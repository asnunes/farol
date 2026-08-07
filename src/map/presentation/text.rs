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
