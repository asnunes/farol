use std::fmt::{self, Display};

use crate::comments::domain::Comment;

/// The comments, for the terminal and for the session reading its own review.
pub struct CommentList<'a>(pub &'a [Comment]);

impl Display for CommentList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return writeln!(f, "No comments yet.");
        }

        for comment in self.0 {
            let lines = match comment.from == comment.to {
                true => format!("{}", comment.from),
                false => format!("{}-{}", comment.from, comment.to),
            };
            writeln!(f, "{}  {}:{}", comment.id, comment.path, lines)?;
            for line in comment.body.lines() {
                writeln!(f, "    {line}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(from: u32, to: u32) -> Comment {
        Comment {
            id: "abc-1".into(),
            path: "src/a.rs".into(),
            from,
            to,
            body: "Why this order?\nIt reads backwards.".into(),
        }
    }

    #[test]
    fn each_comment_says_where_it_is_and_what_it_says() {
        let out = CommentList(&[comment(82, 116)]).to_string();

        assert!(out.contains("abc-1  src/a.rs:82-116"), "{out}");
        assert!(out.contains("    Why this order?"), "{out}");
        assert!(out.contains("    It reads backwards."), "{out}");
    }

    #[test]
    fn a_comment_on_one_line_says_one_line() {
        assert!(
            CommentList(&[comment(9, 9)])
                .to_string()
                .contains("src/a.rs:9\n")
        );
    }

    #[test]
    fn nothing_written_yet_says_so_rather_than_printing_emptiness() {
        assert_eq!(CommentList(&[]).to_string().trim(), "No comments yet.");
    }
}
