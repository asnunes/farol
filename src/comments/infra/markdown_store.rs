use std::path::PathBuf;

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
}

impl MarkdownComments {
    pub fn new(store: &Store) -> Self {
        Self {
            dir: store.comments_dir(),
        }
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.md"))
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
                None => found.unreadable.push(file.display().to_string()),
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
        (dir, MarkdownComments::new(&store))
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
        assert_eq!(found.unreadable.len(), 1);
        assert!(
            found.unreadable[0].ends_with("2.md"),
            "{:?}",
            found.unreadable
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
