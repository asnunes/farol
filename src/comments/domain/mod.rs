mod error;
mod publishing;

pub use error::*;
pub use publishing::*;

use crate::diff::domain::Side;
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
    /// Which side of the diff `from` and `to` are counted on.
    ///
    /// Not a detail of how it is displayed: the same pair of numbers names one
    /// passage on the old side and a different one on the new, and the host is
    /// told the side along with the numbers. A comment without it is a comment
    /// that could land on either.
    pub side: Side,
    pub from: u32,
    pub to: u32,
    /// Markdown, kept as written.
    pub body: String,
    /// Where it lives on the pull request, once it has been published.
    ///
    /// Published is not closed. The question is still open — it just has a
    /// second home now, and the answer will come back there. Closing still
    /// removes the comment; this only records that it left.
    pub published: Option<String>,
}

impl Comment {
    /// What lands on the clipboard: enough for the answer to be read somewhere
    /// else without the file open.
    pub fn quoted(&self) -> String {
        format!("{}\n\n{}", self.at(), self.body.trim())
    }

    /// Where it sits, the way a person would type it to go there.
    ///
    /// The side is written only when it is the old one. Both sides number from
    /// one, so `src/a.rs:12` alone would be two different places — and the
    /// unmarked form has to keep meaning what it has always meant, which is the
    /// file as it now reads.
    pub fn at(&self) -> String {
        let lines = match self.from == self.to {
            true => format!("{}", self.from),
            false => format!("{}-{}", self.from, self.to),
        };
        match self.side {
            Side::New => format!("{}:{lines}", self.path),
            Side::Old => format!("{}:{lines} (old)", self.path),
        }
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
    pub unreadable: Vec<Unreadable>,
}

/// A file that could not be turned into a comment, and everything that can
/// still be said about it.
///
/// What broke is the header, which is the part that says where the comment
/// belongs — so half the time there is no reviewed file left to name. When
/// there is, it is what the reviewer is told, because that is the language the
/// review is read in. When there is not, they get the start of what they wrote,
/// which is how a person recognises their own comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// The file to open to fix it, from where the reviewer is standing.
    pub file: String,
    /// The reviewed file it was written about, when the header still says.
    pub about: Option<String>,
    /// The start of what they wrote, when it does not.
    pub excerpt: Option<String>,
    pub why: Unread,
}

/// What the header is missing. A closed set, so the wording lives with the
/// presentation rather than being built where the file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unread {
    /// No `---` header at all.
    NoHeader,
    /// Nothing saying which file it is about.
    NoPath,
    /// Nothing saying which lines, or something that is not a range.
    NoLines,
    /// A side that is neither `old` nor `new`. A missing one is not this: it
    /// means the new side, which is all farol could write for a long time.
    /// Something else written there is a guess nobody should make, because
    /// guessing wrong puts the comment on the column the reviewer was not
    /// reading.
    BadSide,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(from: u32, to: u32) -> Comment {
        Comment {
            id: "1".into(),
            path: "src/a.rs".into(),
            side: Side::New,
            from,
            to,
            body: "  Why this order?  ".into(),
            published: None,
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

    #[test]
    fn one_on_the_old_side_says_so_where_the_numbers_are() {
        // Line 82 exists on both sides of the same file. Pasted somewhere the
        // diff is not open, the numbers alone would send the reader to the
        // wrong one.
        let removed = Comment {
            side: Side::Old,
            ..comment(82, 116)
        };

        assert_eq!(removed.at(), "src/a.rs:82-116 (old)");
        assert_eq!(
            Comment {
                side: Side::Old,
                ..comment(9, 9)
            }
            .at(),
            "src/a.rs:9 (old)"
        );
    }
}
