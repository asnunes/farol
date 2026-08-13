use crate::error::Result;

/// Something the reviewer wrote about a span of lines.
///
/// Theirs, not the session's: the map carries what the author explained, and
/// this carries what the reader asked back. They sit in different files for the
/// same reason they read differently on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub id: String,
    pub path: String,
    pub from: u32,
    pub to: u32,
    /// Markdown, kept as written.
    pub body: String,
}

impl Comment {
    /// What lands on the clipboard: enough for the answer to be read somewhere
    /// else without the file open.
    pub fn quoted(&self) -> String {
        let lines = match self.from == self.to {
            true => format!("{}", self.from),
            false => format!("{}-{}", self.from, self.to),
        };
        format!("{}:{}\n\n{}", self.path, lines, self.body.trim())
    }
}

/// Where comments are kept. A trait for the same reason the map has one: the
/// use cases should not know that this is a folder of files.
pub trait CommentStore: Send + Sync {
    fn list(&self) -> Result<Found>;
    fn save(&self, comment: &Comment) -> Result<()>;
    /// Whether there was one to close.
    fn close(&self, id: &str) -> Result<bool>;
}

/// What a read of the store turned up.
///
/// The comments are only half of it. These are files a person is invited to
/// open and edit, so one of them will eventually come back malformed — and one
/// bad file must not cost the whole review. It gets skipped, and it gets named:
/// skipping quietly is how a comment somebody wrote disappears without a word,
/// which is worse than the error it was avoiding.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Found {
    pub comments: Vec<Comment>,
    /// Paths of the files that could not be read, so they can be opened and
    /// fixed rather than hunted for.
    pub unreadable: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(from: u32, to: u32) -> Comment {
        Comment {
            id: "1".into(),
            path: "src/a.rs".into(),
            from,
            to,
            body: "  Why this order?  ".into(),
        }
    }

    #[test]
    fn a_copied_comment_carries_where_it_was_about() {
        // Pasted into a chat or an issue, it has to stand on its own.
        assert_eq!(
            comment(82, 116).quoted(),
            "src/a.rs:82-116\n\nWhy this order?"
        );
    }

    #[test]
    fn a_comment_on_one_line_says_one_line() {
        assert_eq!(comment(82, 82).quoted(), "src/a.rs:82\n\nWhy this order?");
    }
}
