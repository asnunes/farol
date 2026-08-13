use std::fmt::{self, Display};

use crate::comments::domain::Found;

/// The comments, for the terminal and for the session reading its own review.
pub struct CommentList<'a>(pub &'a Found);

impl Display for CommentList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.comments.is_empty() && self.0.unreadable.is_empty() {
            return writeln!(f, "No comments yet.");
        }

        for comment in &self.0.comments {
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

        // Loudly, and by name. A file that cannot be read is a comment somebody
        // wrote and can no longer see, and the only way to get it back is to
        // open the file — so the file is what this says.
        if !self.0.unreadable.is_empty() {
            let n = self.0.unreadable.len();
            let s = match n == 1 {
                true => "",
                false => "s",
            };
            writeln!(f, "{n} comment file{s} could not be read:")?;
            for path in &self.0.unreadable {
                writeln!(f, "    {path}")?;
            }
            writeln!(
                f,
                "The header needs `path:` and `lines:` between two --- lines."
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::Comment;

    fn comment(from: u32, to: u32) -> Comment {
        Comment {
            id: "abc-1".into(),
            path: "src/a.rs".into(),
            from,
            to,
            body: "Why this order?\nIt reads backwards.".into(),
        }
    }

    fn found(comments: Vec<Comment>, unreadable: Vec<&str>) -> Found {
        Found {
            comments,
            unreadable: unreadable.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn each_comment_says_where_it_is_and_what_it_says() {
        let out = CommentList(&found(vec![comment(82, 116)], vec![])).to_string();

        assert!(out.contains("abc-1  src/a.rs:82-116"), "{out}");
        assert!(out.contains("    Why this order?"), "{out}");
        assert!(out.contains("    It reads backwards."), "{out}");
    }

    #[test]
    fn a_comment_on_one_line_says_one_line() {
        let out = CommentList(&found(vec![comment(9, 9)], vec![])).to_string();

        assert!(out.contains("src/a.rs:9\n"), "{out}");
    }

    #[test]
    fn a_file_that_could_not_be_read_is_named_and_counted() {
        // Silence here reads as "you never wrote that comment", which is the
        // one thing the reviewer cannot recover from.
        let out = CommentList(&found(vec![], vec![".git/farol/b/comments/x.md"])).to_string();

        assert!(out.contains("1 comment file could not be read"), "{out}");
        assert!(out.contains(".git/farol/b/comments/x.md"), "{out}");
        assert!(
            out.contains("path:"),
            "it has to say what a header needs: {out}"
        );
    }

    #[test]
    fn several_of_them_read_as_several() {
        let out = CommentList(&found(vec![], vec!["a.md", "b.md"])).to_string();

        assert!(out.contains("2 comment files could not be read"), "{out}");
    }

    #[test]
    fn nothing_written_yet_says_so_rather_than_printing_emptiness() {
        assert_eq!(
            CommentList(&Found::default()).to_string().trim(),
            "No comments yet."
        );
    }
}
