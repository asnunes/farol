use std::path::PathBuf;

use crate::comments::domain::{Comment, CommentStore};
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
    fn list(&self) -> Result<Vec<Comment>> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Ok(Vec::new());
        };

        let mut found: Vec<Comment> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
            .filter_map(|e| {
                let id = e.path().file_stem()?.to_string_lossy().into_owned();
                parse(&id, &std::fs::read_to_string(e.path()).ok()?)
            })
            .collect();

        // By id, which is the second it was written: the thread reads in the
        // order it happened.
        found.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(found)
    }

    fn save(&self, comment: &Comment) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.file(&comment.id), render(comment))?;
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<bool> {
        match std::fs::remove_file(self.file(id)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Error::from(e)),
        }
    }
}

fn render(comment: &Comment) -> String {
    format!(
        "---\npath: {}\nlines: {}-{}\nresolved: {}\n---\n\n{}\n",
        comment.path,
        comment.from,
        comment.to,
        comment.resolved,
        comment.body.trim()
    )
}

/// The header, then the prose. Written by hand rather than with a yaml crate:
/// four keys do not justify a dependency, and a file a person edited by hand
/// and got slightly wrong should be skipped, not fatal.
fn parse(id: &str, raw: &str) -> Option<Comment> {
    let rest = raw.strip_prefix("---\n")?;
    let (head, body) = rest.split_once("\n---\n")?;

    let mut path = None;
    let mut lines = None;
    let mut resolved = false;
    for line in head.lines() {
        let (key, value) = line.split_once(':')?;
        match key.trim() {
            "path" => path = Some(value.trim().to_string()),
            "lines" => lines = Some(value.trim().to_string()),
            "resolved" => resolved = value.trim() == "true",
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
        resolved,
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
            resolved: false,
        }
    }

    #[test]
    fn a_comment_comes_back_the_way_it_went_in() {
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();

        assert_eq!(store.list().unwrap(), vec![comment("1")]);
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
    fn closing_one_keeps_it() {
        // Resolved is not deleted: the thread is the record of what was asked.
        let (_dir, store) = store();
        store
            .save(&Comment {
                resolved: true,
                ..comment("1")
            })
            .unwrap();

        assert!(store.list().unwrap()[0].resolved);
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

        assert_eq!(store.list().unwrap()[0].from, 9);
        assert_eq!(store.list().unwrap()[0].to, 9);
    }

    #[test]
    fn a_file_somebody_edited_into_nonsense_is_skipped_rather_than_fatal() {
        // These are files a person is invited to open. One of them being wrong
        // should cost that one comment, not the review.
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();
        std::fs::write(store.file("2"), "not a comment at all").unwrap();

        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn removing_says_whether_there_was_anything_to_remove() {
        let (_dir, store) = store();
        store.save(&comment("1")).unwrap();

        assert!(store.remove("1").unwrap());
        assert!(!store.remove("1").unwrap());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn they_come_back_in_the_order_they_were_written() {
        let (_dir, store) = store();
        store.save(&comment("2")).unwrap();
        store.save(&comment("1")).unwrap();

        let ids: Vec<_> = store.list().unwrap().into_iter().map(|c| c.id).collect();
        assert_eq!(ids, ["1", "2"]);
    }
}
