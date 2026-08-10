//! Shaping the small things every report does the same way.
//!
//! Shared so two reports cannot disagree about how long a sha is or how a
//! wrapped note lines up.

use std::fmt::Write as _;

use crate::map::domain::WORKING;

pub(super) fn short(sha: &str) -> String {
    if sha == WORKING {
        sha.to_string()
    } else {
        sha.chars().take(7).collect()
    }
}

/// Keep wrapped prose lined up under its label instead of falling back to
/// column zero, where it would read as a new field.
pub(super) fn indent_rest(text: &str, spaces: usize) -> String {
    let mut out = String::new();
    let pad = " ".repeat(spaces);
    for (i, line) in text.trim().lines().enumerate() {
        if i > 0 {
            let _ = write!(out, "\n{pad}");
        }
        let _ = write!(out, "{line}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sha_is_shortened_to_what_a_reader_can_hold() {
        assert_eq!(short("a3f1e9c1234567890"), "a3f1e9c");
    }

    #[test]
    fn a_sha_shorter_than_the_cut_is_left_alone() {
        assert_eq!(short("abc"), "abc");
    }

    #[test]
    fn the_working_tree_marker_is_printed_whole() {
        // Cutting it to seven characters would turn a word into gibberish.
        assert_eq!(short(WORKING), WORKING);
    }

    #[test]
    fn a_single_line_of_prose_is_returned_as_it_stands() {
        assert_eq!(indent_rest("one line", 4), "one line");
    }

    #[test]
    fn wrapped_prose_lines_up_under_its_label() {
        // Falling back to column zero would make the second line read as a new
        // field rather than the rest of this one.
        assert_eq!(
            indent_rest("first\nsecond\nthird", 2),
            "first\n  second\n  third"
        );
    }

    #[test]
    fn surrounding_blank_space_does_not_become_an_empty_line() {
        // Prose arrives from a shell argument and often carries a stray
        // newline; printing it would open the field with a blank line.
        assert_eq!(indent_rest("\n  padded  \n", 4), "padded");
    }

    #[test]
    fn a_blank_line_inside_the_prose_is_kept_and_still_lines_up() {
        // The author put it there to separate paragraphs.
        assert_eq!(indent_rest("first\n\nsecond", 2), "first\n  \n  second");
    }
}
