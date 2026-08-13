use std::path::{Path, PathBuf};

use crate::comments::domain::{Comment, CommentStore, Found};
use crate::error::{Error, Result};
use crate::shared::paths::Store;

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
            let read = std::fs::read_to_string(&file)
                .ok()
                .zip(file.file_stem())
                .and_then(|(raw, id)| parse(&id.to_string_lossy(), &raw));

            match read {
                Some(comment) => found.comments.push(comment),
                None => found.unreadable.push(self.named(&file)),
            }
        }

        // By id, which is the moment it was written: the thread reads in the
        // order it happened. The unreadable ones by name, for the same reason
        // any list of files is sorted — so it reads the same twice running.
        found.comments.sort_by(|a, b| a.id.cmp(&b.id));
        found.unreadable.sort();
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

fn render(comment: &Comment) -> String {
    format!(
        "---\npath: {}\nlines: {}-{}\n---\n\n{}\n",
        comment.path,
        comment.from,
        comment.to,
        comment.body.trim()
    )
}

/// The header, then the prose. Written by hand rather than with a yaml crate:
/// two keys do not justify a dependency, and a file a person edited by hand and
/// got slightly wrong should be skipped, not fatal.
///
/// A key it does not know is ignored rather than refused, which is what lets an
/// older file — one still carrying `resolved:` from when closing kept the
/// comment — be read without a migration.
fn parse(id: &str, raw: &str) -> Option<Comment> {
    let rest = raw.strip_prefix("---\n")?;
    let (head, body) = rest.split_once("\n---\n")?;

    let mut path = None;
    let mut lines = None;
    for line in head.lines() {
        let (key, value) = line.split_once(':')?;
        match key.trim() {
            "path" => path = Some(value.trim().to_string()),
            "lines" => lines = Some(value.trim().to_string()),
            _ => {}
        }
    }

    let lines = lines?;
    let (from, to) = lines.split_once('-').unwrap_or((&lines, &lines));

    Some(Comment {
        id: id.to_string(),
        path: path?,
        from: from.trim().parse().ok()?,
        to: to.trim().parse().ok()?,
        body: body.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
            from: 82,
            to: 116,
            body: "Why **this** order?".into(),
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
        assert_eq!(
            found.unreadable,
            vec!["farol/feature-x/comments/2.md"],
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
            found.unreadable,
            vec!["../../main/.git/farol/feature-x/comments/1.md"]
        );
    }

    #[test]
    fn every_way_of_breaking_the_header_is_reported_rather_than_swallowed() {
        // The four a person actually produces by editing the file: the header
        // gone, either key gone, and a stray line with no colon in it.
        let (_dir, store) = store();
        let broken = [
            ("a", "no header at all, just prose\n"),
            ("b", "---\nlines: 1-2\n---\n\nWhy?\n"),
            ("c", "---\npath: src/a.rs\n---\n\nWhy?\n"),
            (
                "d",
                "---\npath: src/a.rs\nrascunho\nlines: 1-2\n---\n\nWhy?\n",
            ),
        ];
        for (id, raw) in broken {
            std::fs::create_dir_all(&store.dir).unwrap();
            std::fs::write(store.file(id), raw).unwrap();
        }

        let found = store.list().unwrap();

        assert!(found.comments.is_empty());
        assert_eq!(found.unreadable.len(), 4, "{:?}", found.unreadable);
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
