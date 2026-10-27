//! The places in the code an answer points at.

use serde::{Deserialize, Serialize};

use crate::diff::domain::Side;

/// A place an answer cites, as the session wrote it: `src/retry.rs:90-91` in
/// backticks, with `(old)` after it for a line the change removed.
///
/// Only code spans are read. A path in running prose is too easy to mistake,
/// and the prompt asks for the backticks, which is what makes the citation
/// cheap to find on both sides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    /// The code span exactly as written, which is what the screen looks for.
    pub text: String,
    pub path: String,
    pub side: Side,
    pub from: u32,
    pub to: u32,
    /// Whether it was checked against the review, and so can be a link.
    #[serde(default)]
    pub linked: bool,
}

impl Citation {
    /// Every citation in a text, unchecked.
    pub fn all_in(text: &str) -> Vec<Self> {
        text.split('`')
            .skip(1)
            .step_by(2)
            .filter_map(Self::parse)
            .collect()
    }

    fn parse(span: &str) -> Option<Self> {
        let text = span.to_string();
        let span = span.trim();
        let (rest, side) = match span.strip_suffix("(old)") {
            Some(rest) => (rest.trim_end(), Side::Old),
            None => (span, Side::New),
        };
        let (path, range) = rest.rsplit_once(':')?;
        if path.is_empty() || path.contains(char::is_whitespace) {
            return None;
        }
        if !(path.contains('/') || path.contains('.')) {
            return None;
        }
        let (from, to) = match range.split_once('-') {
            Some((a, b)) => (a.parse().ok()?, b.parse().ok()?),
            None => {
                let at = range.parse().ok()?;
                (at, at)
            }
        };
        if from == 0 || to < from {
            return None;
        }
        Some(Self {
            text,
            path: path.to_string(),
            side,
            from,
            to,
            linked: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citations_are_read_from_code_spans_only() {
        let found = Citation::all_in(
            "See `src/retry.rs:90-91`, `tests/retry.rs:14` and `src/old.rs:3-4 (old)`; not src/a.rs:1 or `RetryPolicy`.",
        );

        let seen: Vec<_> = found
            .iter()
            .map(|c| (c.path.as_str(), c.side, c.from, c.to))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("src/retry.rs", Side::New, 90, 91),
                ("tests/retry.rs", Side::New, 14, 14),
                ("src/old.rs", Side::Old, 3, 4),
            ]
        );
    }

    #[test]
    fn a_span_that_only_looks_like_a_citation_is_left_alone() {
        assert!(Citation::all_in("`a:1` `x.rs:0` `x.rs:5-2` `std::fmt`").is_empty());
    }
}
