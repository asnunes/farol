use std::fmt::{self, Display};

use crate::comments::domain::{Found, Unread, Unreadable};

/// The comments, for the terminal and for the session reading its own review.
pub struct CommentList<'a>(pub &'a Found);

impl Display for CommentList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.comments.is_empty() && self.0.unreadable.is_empty() {
            return writeln!(f, "No comments yet.");
        }

        for comment in &self.0.comments {
            // `at` and not the three fields laid out again: it is the one place
            // that knows a span on the old side has to say so, and a second
            // spelling here would be the one that forgets.
            writeln!(f, "{}  {}", comment.id, comment.at())?;
            for line in comment.body.lines() {
                writeln!(f, "    {line}")?;
            }
            writeln!(f)?;
        }

        // Named the way the review is read, not the way it is stored: the file
        // it was written about when the header still says, and the start of
        // the prose when it does not. The `.md` comes last, because opening it
        // is the fix rather than the point.
        if !self.0.unreadable.is_empty() {
            let n = self.0.unreadable.len();
            let s = match n == 1 {
                true => "",
                false => "s",
            };
            writeln!(f, "{n} comment{s} could not be read:")?;
            for one in &self.0.unreadable {
                writeln!(f, "    {}", headline(one))?;
                writeln!(f, "    {}", says(one.why))?;
                writeln!(f, "    {}\n", one.file)?;
            }
        }
        Ok(())
    }
}

/// What the reviewer knows it by: the file it was about, or failing that, the
/// first line of what they wrote.
fn headline(one: &Unreadable) -> String {
    match (&one.about, &one.excerpt) {
        (Some(path), _) => path.clone(),
        (None, Some(text)) => format!("\"{text}\""),
        (None, None) => "an empty comment".to_string(),
    }
}

/// Why a comment could not be read, in one clause.
///
/// Shared with the view the browser gets, so the terminal and the screen say
/// the same thing about the same file.
pub fn says(why: Unread) -> &'static str {
    match why {
        Unread::NoHeader => "its header is gone, so nothing says where it belongs",
        Unread::NoPath => "the header no longer says which file it is about",
        Unread::NoLines => "the header no longer says which lines",
        Unread::BadSide => "the header says a side that is neither 'old' nor 'new'",
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
            side: crate::diff::domain::Side::New,
            from,
            to,
            body: "Why this order?\nIt reads backwards.".into(),
            published: None,
        }
    }

    fn found(comments: Vec<Comment>, unreadable: Vec<Unreadable>) -> Found {
        Found {
            comments,
            unreadable,
        }
    }

    fn broken(about: Option<&str>, excerpt: Option<&str>, why: Unread) -> Unreadable {
        Unreadable {
            file: ".git/farol/b/comments/x.md".into(),
            about: about.map(String::from),
            excerpt: excerpt.map(String::from),
            why,
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
    fn one_on_the_old_side_says_which_side_it_is_on() {
        // The list is read by a session as well as by a person, and both of
        // them would otherwise go looking at line 82 of the wrong file.
        let out = CommentList(&found(
            vec![Comment {
                side: crate::diff::domain::Side::Old,
                ..comment(82, 116)
            }],
            vec![],
        ))
        .to_string();

        assert!(out.contains("abc-1  src/a.rs:82-116 (old)"), "{out}");
    }

    #[test]
    fn one_that_still_knows_its_file_is_named_by_the_file() {
        // The review is read in file names, so that is what the reviewer is
        // told whenever the header still carries one.
        let out = CommentList(&found(
            vec![],
            vec![broken(Some("src/a.rs"), Some("Why?"), Unread::NoLines)],
        ))
        .to_string();

        assert!(out.contains("src/a.rs"), "{out}");
        assert!(out.contains("no longer says which lines"), "{out}");
        assert!(
            out.contains(".git/farol/b/comments/x.md"),
            "the fix is to open it: {out}"
        );
    }

    #[test]
    fn one_that_lost_its_file_is_named_by_what_it_says() {
        // Nothing left to place it by, so the reviewer gets their own words
        // back — which is how a person recognises a comment they wrote.
        let out = CommentList(&found(
            vec![],
            vec![broken(None, Some("Por que essa ordem?"), Unread::NoHeader)],
        ))
        .to_string();

        assert!(out.contains("\"Por que essa ordem?\""), "{out}");
        assert!(out.contains("nothing says where it belongs"), "{out}");
    }

    #[test]
    fn several_of_them_read_as_several() {
        let out = CommentList(&found(
            vec![],
            vec![
                broken(None, Some("a"), Unread::NoHeader),
                broken(None, Some("b"), Unread::NoPath),
            ],
        ))
        .to_string();

        assert!(out.contains("2 comments could not be read"), "{out}");
    }

    #[test]
    fn nothing_written_yet_says_so_rather_than_printing_emptiness() {
        assert_eq!(
            CommentList(&Found::default()).to_string().trim(),
            "No comments yet."
        );
    }
}
