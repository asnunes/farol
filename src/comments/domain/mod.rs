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
    fn list(&self) -> Result<Vec<Comment>>;
    fn save(&self, comment: &Comment) -> Result<()>;
    /// Whether there was one to close.
    fn close(&self, id: &str) -> Result<bool>;
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
