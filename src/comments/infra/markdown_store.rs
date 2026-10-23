use std::path::{Path, PathBuf};

use crate::comments::domain::{Comment, CommentStore, Found};
use crate::error::{Error, Result};
use crate::shared::paths::Store;

mod format;

use format::{parse, render};

/// One markdown file per comment, under the branch's own folder.
///
/// Markdown because the body is prose the reviewer wrote and may want to edit
/// in an editor, and one file each because that makes writing, deleting and
/// reading back a single comment three lines apiece. A single file holding all
/// of them would need a parser that can also put it back.
pub struct MarkdownComments {
    dir: PathBuf,
    /// What a file that could not be read is named relative to.
    ///
    /// The worktree the reviewer is standing in, because the name is there to
    /// be opened and that is where they are standing when they open it. The
    /// absolute form would be mostly a prefix they already know, wrapped over
    /// two lines in a terminal and squeezed into a tooltip on screen.
    root: PathBuf,
}

impl MarkdownComments {
    pub fn new(store: &Store, root: &Path) -> Self {
        Self {
            dir: store.comments_dir(),
            root: root.to_path_buf(),
        }
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.md"))
    }

    /// Where the file sits, from where the reviewer is standing.
    ///
    /// A linked worktree keeps its git dir outside the tree it belongs to, so
    /// this climbs through `..` rather than giving up and printing the whole
    /// path: `../../.git/farol/…` is still something to paste, and the
    /// absolute form is a prefix nobody needed to be told.
    fn named(&self, file: &Path) -> String {
        let mut here = self.root.components().peekable();
        let mut there = file.components().peekable();
        while here.peek().is_some() && here.peek() == there.peek() {
            here.next();
            there.next();
        }

        let mut out = PathBuf::new();
        here.for_each(|_| out.push(".."));
        there.for_each(|c| out.push(c));
        out.display().to_string()
    }
}

impl CommentStore for MarkdownComments {
    fn list(&self) -> Result<Found> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Ok(Found::default());
        };

        let mut found = Found::default();
        for file in entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
        {
            let Some((raw, id)) = std::fs::read_to_string(&file).ok().zip(file.file_stem()) else {
                continue;
            };

            match parse(&id.to_string_lossy(), self.named(&file), &raw) {
                Ok(comment) => found.comments.push(comment),
                Err(unread) => found.unreadable.push(unread),
            }
        }

        // By id, which is the moment it was written: the questions read in the
        // order they were asked. The unreadable ones by name, for the same
        // reason any list of files is sorted — so it reads the same twice
        // running.
        found.comments.sort_by(|a, b| a.id.cmp(&b.id));
        found.unreadable.sort_by(|a, b| a.file.cmp(&b.file));
        Ok(found)
    }

    fn save(&self, comment: &Comment) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.file(&comment.id), render(comment))?;
        Ok(())
    }

    fn close(&self, id: &str) -> Result<bool> {
        match std::fs::remove_file(self.file(id)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Error::from(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::{Author, PendingRef, Purpose, Unread};
    use crate::diff::domain::Side;

    fn store() -> (tempfile::TempDir, MarkdownComments) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path(), "feature/x");
        let comments = MarkdownComments::new(&store, dir.path());
        (dir, comments)
    }

    fn comment(id: &str) -> Comment {
        Comment {
            id: id.into(),
            path: "src/a.rs".into(),
            side: Side::New,
            from: 82,
            to: 116,
            body: "Why **this** order?".into(),
            pending: None,
            conversation: Default::default(),
        }
    }

    #[test]
    fn a_comment_comes_back_the_way_it_went_in() {
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();

        assert_eq!(store.list().unwrap().comments, vec![comment("1")]);
    }

    #[test]
    fn the_body_stays_markdown_a_person_could_have_typed() {
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();

        let raw = std::fs::read_to_string(store.file("1")).unwrap();
        assert!(raw.ends_with("Why **this** order?\n"), "{raw}");
        assert!(raw.starts_with("---\npath: src/a.rs\n"), "{raw}");
    }

    #[test]
    fn the_pending_comment_and_what_the_agent_owes_come_back_with_it() {
        let (_dir, store) = store();
        let mut one = comment("1");
        one.pending_written(PendingRef {
            id: "PRRC_kwDO".into(),
            by: Author::Agent("Claude Code".into()),
        });
        one.sent("Claude Code", "17", Purpose::Pending);
        store.save(&one).unwrap();

        let back = &store.list().unwrap().comments[0];
        assert_eq!(back, &one);
        assert_eq!(
            back.conversation.waiting.as_ref().unwrap().purpose,
            Purpose::Pending,
            "a regenerate in flight must not read back as an answer owed"
        );
    }

    #[test]
    fn a_comment_published_by_the_first_release_candidate_reads_as_a_thread() {
        // rc.1 sent the comment itself with the review. That copy cannot be
        // edited from here, so the thread comes back without a pending one.
        let (_dir, store) = store();
        std::fs::create_dir_all(&store.dir).unwrap();
        std::fs::write(
            store.file("1"),
            "---\npath: src/a.rs\nlines: 82-116\npublished: https://github.com/o/r/pull/1#r1\n---\n\nWhy **this** order?\n",
        )
        .unwrap();

        assert_eq!(store.list().unwrap().comments, vec![comment("1")]);
    }

    #[test]
    fn which_side_it_was_written_on_survives_the_round_trip() {
        // The file outlives the session, and reading it back on the wrong side
        // would move the comment onto code it was never about.
        let (_dir, store) = store();
        store
            .save(&Comment {
                side: Side::Old,
                ..comment("1")
            })
            .unwrap();

        let raw = std::fs::read_to_string(store.file("1")).unwrap();
        assert!(raw.contains("side: old"), "{raw}");
        assert_eq!(store.list().unwrap().comments[0].side, Side::Old);
    }

    #[test]
    fn a_comment_written_before_sides_existed_is_about_the_file_as_it_now_reads() {
        // Every comment farol could write back then was on the new side, so a
        // file with no `side:` is not ambiguous — reading it as `new` is
        // reading it correctly, and nothing has to be migrated.
        let (_dir, store) = store();
        std::fs::create_dir_all(&store.dir).unwrap();
        std::fs::write(
            store.file("1"),
            "---\npath: src/a.rs\nlines: 82-116\n---\n\nWhy **this** order?\n",
        )
        .unwrap();

        assert_eq!(store.list().unwrap().comments, vec![comment("1")]);
    }

    #[test]
    fn a_side_that_is_neither_is_skipped_and_named_rather_than_guessed_at() {
        // `left` is what somebody types who knows GitHub's vocabulary. Taking
        // it for one side or the other is how a question lands on the column
        // they were not reading; saying so costs them one file to fix.
        let (_dir, store) = store();
        std::fs::create_dir_all(&store.dir).unwrap();
        std::fs::write(
            store.file("1"),
            "---\npath: src/a.rs\nside: left\nlines: 82-116\n---\n\nWhy?\n",
        )
        .unwrap();

        let found = store.list().unwrap();

        assert!(found.comments.is_empty());
        assert_eq!(found.unreadable[0].why, Unread::BadSide);
        assert_eq!(found.unreadable[0].about.as_deref(), Some("src/a.rs"));
    }

    #[test]
    fn a_file_written_before_publishing_existed_reads_as_unpublished() {
        // The reviewer's comments outlive the version of farol that wrote them,
        // and a missing key is not a broken file — it is a comment that has not
        // left yet.
        let (_dir, store) = store();
        std::fs::create_dir_all(store.file("1").parent().unwrap()).unwrap();
        std::fs::write(
            store.file("1"),
            "---\npath: src/a.rs\nlines: 82-116\n---\n\nWhy **this** order?\n",
        )
        .unwrap();

        assert_eq!(store.list().unwrap().comments, vec![comment("1")]);
    }

    #[test]
    fn a_file_still_carrying_a_key_from_an_older_farol_is_read_anyway() {
        // `resolved:` was written into every comment while closing kept it.
        // Refusing those files would lose comments somebody is mid-review on.
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();
        std::fs::write(
            store.file("2"),
            "---\npath: src/a.rs\nlines: 4-6\nresolved: false\n---\n\nWhy?\n",
        )
        .unwrap();

        let all = store.list().unwrap().comments;
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].body, "Why?");
    }

    #[test]
    fn a_comment_on_a_single_line_survives_the_round_trip() {
        let (_dir, store) = store();
        store
            .save(&Comment {
                from: 9,
                to: 9,
                ..comment("1")
            })
            .unwrap();

        assert_eq!(store.list().unwrap().comments[0].from, 9);
        assert_eq!(store.list().unwrap().comments[0].to, 9);
    }

    #[test]
    fn a_file_somebody_edited_into_nonsense_is_skipped_and_named() {
        // These are files a person is invited to open. One of them being wrong
        // should cost that one comment, not the review — and it has to be said
        // out loud, or a comment they wrote is gone with no way to notice.
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();
        std::fs::write(store.file("2"), "not a comment at all").unwrap();

        let found = store.list().unwrap();
        assert_eq!(found.comments.len(), 1);
        assert_eq!(found.unreadable.len(), 1);
        assert_eq!(
            found.unreadable[0].file, "farol/feature-x/comments/2.md",
            "named from the root of the worktree, to be pasted into an editor"
        );
    }

    #[test]
    fn a_store_outside_the_worktree_climbs_out_rather_than_going_absolute() {
        // A linked worktree keeps its git dir under the repository it was cut
        // from, so the store is not inside the tree the reviewer is standing
        // in. `..` from where they are still beats a path from the root of the
        // disk.
        let repo = tempfile::tempdir().unwrap();
        let store = Store::new(&repo.path().join("main/.git"), "feature/x");
        let worktree = repo.path().join("trees/feature-x");
        let comments = MarkdownComments::new(&store, &worktree);
        std::fs::create_dir_all(store.comments_dir()).unwrap();
        std::fs::write(store.comments_dir().join("1.md"), "broken").unwrap();

        let found = comments.list().unwrap();

        assert_eq!(
            found.unreadable[0].file,
            "../../main/.git/farol/feature-x/comments/1.md"
        );
    }

    #[test]
    fn a_broken_header_is_reported_in_the_terms_the_review_is_read_in() {
        // Each way a person actually breaks it by editing, and what is left to
        // tell them by: the reviewed file when the header still names it, and
        // the first line of their own prose when it does not.
        let (_dir, store) = store();
        std::fs::create_dir_all(&store.dir).unwrap();
        for (id, raw) in [
            ("a", "no header at all, just the question I asked\n"),
            ("b", "---\nlines: 1-2\n---\n\nWhy this order?\n"),
            ("c", "---\npath: src/a.rs\n---\n\nWhy this order?\n"),
        ] {
            std::fs::write(store.file(id), raw).unwrap();
        }

        let broken = store.list().unwrap().unreadable;

        assert_eq!(broken.len(), 3);
        assert_eq!(broken[0].why, Unread::NoHeader);
        assert_eq!(broken[0].about, None);
        assert_eq!(
            broken[0].excerpt.as_deref(),
            Some("no header at all, just the question I asked")
        );

        assert_eq!(broken[1].why, Unread::NoPath);
        assert_eq!(broken[1].about, None);
        assert_eq!(broken[1].excerpt.as_deref(), Some("Why this order?"));

        // The one case where the review still has a name for it.
        assert_eq!(broken[2].why, Unread::NoLines);
        assert_eq!(broken[2].about.as_deref(), Some("src/a.rs"));
    }

    #[test]
    fn a_stray_line_in_the_header_costs_nothing() {
        // A draft, a blank line, a key from an older farol. None of them is a
        // reason to lose the comment underneath.
        let (_dir, store) = store();
        std::fs::create_dir_all(&store.dir).unwrap();
        std::fs::write(
            store.file("1"),
            "---\npath: src/a.rs\nrascunho\n\nresolved: false\nlines: 4-6\n---\n\nWhy?\n",
        )
        .unwrap();

        let found = store.list().unwrap();

        assert!(found.unreadable.is_empty(), "{:?}", found.unreadable);
        assert_eq!(found.comments[0].from, 4);
        assert_eq!(found.comments[0].body, "Why?");
    }

    #[test]
    fn a_long_comment_is_cut_short_enough_to_recognise() {
        let (_dir, store) = store();
        std::fs::create_dir_all(&store.dir).unwrap();
        std::fs::write(store.file("1"), format!("no header\n{}", "a".repeat(200))).unwrap();

        let excerpt = store.list().unwrap().unreadable[0].excerpt.clone().unwrap();

        assert_eq!(excerpt, "no header");
    }

    #[test]
    fn a_store_nobody_has_written_to_yet_is_empty_rather_than_broken() {
        let (_dir, store) = store();

        assert_eq!(store.list().unwrap(), Found::default());
    }

    #[test]
    fn closing_says_whether_there_was_anything_to_close() {
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();

        assert!(store.close("1").unwrap());
        assert!(!store.close("1").unwrap());
        assert!(store.list().unwrap().comments.is_empty());
    }

    #[test]
    fn they_come_back_in_the_order_they_were_written() {
        let (_dir, store) = store();
        store.save(&comment("2")).unwrap();
        store.save(&comment("1")).unwrap();

        let ids: Vec<_> = store
            .list()
            .unwrap()
            .comments
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, ["1", "2"]);
    }
}
