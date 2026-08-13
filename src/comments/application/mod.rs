use std::sync::Arc;

use crate::comments::domain::{Comment, CommentStore};
use crate::error::{Error, Result};

/// Everything that can be done to the reviewer's comments.
#[derive(Clone)]
pub struct Comments {
    store: Arc<dyn CommentStore>,
}

impl Comments {
    pub fn new(store: Arc<dyn CommentStore>) -> Self {
        Self { store }
    }

    pub fn all(&self) -> Result<Vec<Comment>> {
        self.store.list()
    }

    pub fn add(&self, path: &str, from: u32, to: u32, body: &str) -> Result<Comment> {
        if body.trim().is_empty() {
            return Err(Error::msg("a comment with no text says nothing"));
        }

        let comment = Comment {
            id: fresh_id(),
            path: path.to_string(),
            from,
            to,
            body: body.trim().to_string(),
            resolved: false,
        };
        self.store.save(&comment)?;
        Ok(comment)
    }

    /// Close one, or open it again. Closing keeps it: the thread is the record
    /// of what was asked and answered.
    pub fn resolve(&self, id: &str, resolved: bool) -> Result<Comment> {
        let mut comment = self.one(id)?;
        comment.resolved = resolved;
        self.store.save(&comment)?;
        Ok(comment)
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        match self.store.remove(id)? {
            true => Ok(()),
            false => Err(Self::unknown(id)),
        }
    }

    fn one(&self, id: &str) -> Result<Comment> {
        self.all()?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| Self::unknown(id))
    }

    fn unknown(id: &str) -> Error {
        Error::msg(format!(
            "no comment with id {id} — `farol comment list` shows them"
        ))
    }
}

/// An id that survives leaving this machine.
///
/// Comments are meant to travel: the reviewer's come back to the author, and
/// two sets get read together. A counter would collide the moment that happens,
/// so this is the clock plus the process, which two machines cannot both
/// produce.
fn fresh_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{now:x}-{:x}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::infra::MarkdownComments;
    use crate::shared::paths::Store;

    fn comments() -> (tempfile::TempDir, Comments) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path(), "feature/x");
        (dir, Comments::new(Arc::new(MarkdownComments::new(&store))))
    }

    #[test]
    fn a_comment_is_written_and_read_back() {
        let (_dir, comments) = comments();
        comments
            .add("src/a.rs", 82, 116, "Why this order?")
            .unwrap();

        let all = comments.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].body, "Why this order?");
        assert!(!all[0].resolved);
    }

    #[test]
    fn an_empty_comment_is_refused() {
        let (_dir, comments) = comments();

        assert!(comments.add("src/a.rs", 1, 1, "   \n ").is_err());
        assert!(comments.all().unwrap().is_empty());
    }

    #[test]
    fn closing_a_comment_keeps_it_on_the_page() {
        let (_dir, comments) = comments();
        let one = comments.add("src/a.rs", 1, 1, "Why?").unwrap();

        comments.resolve(&one.id, true).unwrap();

        let all = comments.all().unwrap();
        assert_eq!(all.len(), 1, "closed is not deleted");
        assert!(all[0].resolved);
    }

    #[test]
    fn a_closed_comment_can_be_opened_again() {
        let (_dir, comments) = comments();
        let one = comments.add("src/a.rs", 1, 1, "Why?").unwrap();
        comments.resolve(&one.id, true).unwrap();

        comments.resolve(&one.id, false).unwrap();

        assert!(!comments.all().unwrap()[0].resolved);
    }

    #[test]
    fn naming_a_comment_that_is_not_there_says_how_to_find_out() {
        let (_dir, comments) = comments();

        let err = comments.remove("nope").unwrap_err().to_string();
        assert!(err.contains("nope"), "{err}");
        assert!(err.contains("comment list"), "{err}");
    }

    #[test]
    fn two_comments_written_together_do_not_share_an_id() {
        // They are written to travel: the reviewer's come back to the author
        // and the two sets are read side by side.
        let (_dir, comments) = comments();
        comments.add("src/a.rs", 1, 1, "one").unwrap();
        comments.add("src/a.rs", 2, 2, "two").unwrap();

        let all = comments.all().unwrap();
        assert_ne!(all[0].id, all[1].id);
    }
}
