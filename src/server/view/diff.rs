//! One file's diff, and stretches of a file the diff did not print.

use serde::Serialize;

use crate::diff::domain::{FileDiff, Hunk, Line, LineKind};

/// One file's diff, as the pane reads it.
///
/// A view like every other answer rather than the domain's own `FileDiff`
/// serialised whole. Two reasons: the names arrive in the one convention the
/// browser uses, and what the domain grows next is not published by accident.
/// `old_path` and `new_content_hash` are on the stored type and stop here,
/// because the screen has never had a use for either.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffView {
    pub path: String,
    pub status: &'static str,
    pub hunks: Vec<HunkView>,
    /// git will not diff this file: binary content, or `-diff` in
    /// `.gitattributes`. There are no hunks, and the screen says so instead of
    /// rendering decoded bytes as if they were code.
    pub binary: bool,
    pub additions: u32,
    pub deletions: u32,
    /// How long the file is after the change, which is where the last gap the
    /// reader can open ends.
    pub line_count: u32,
}

impl FileDiffView {
    pub fn of(diff: &FileDiff) -> Self {
        Self {
            path: diff.path.clone(),
            status: diff.status.label(),
            hunks: diff.hunks.iter().map(HunkView::of).collect(),
            binary: diff.binary,
            additions: diff.additions,
            deletions: diff.deletions,
            line_count: diff.line_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HunkView {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<LineView>,
}

impl HunkView {
    fn of(hunk: &Hunk) -> Self {
        Self {
            old_start: hunk.old_start,
            old_lines: hunk.old_lines,
            new_start: hunk.new_start,
            new_lines: hunk.new_lines,
            lines: hunk.lines.iter().map(LineView::of).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineView {
    pub kind: &'static str,
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
    pub content: String,
}

impl LineView {
    fn of(line: &Line) -> Self {
        Self {
            kind: match line.kind {
                LineKind::Context => "context",
                LineKind::Added => "added",
                LineKind::Removed => "removed",
            },
            old_number: line.old_number,
            new_number: line.new_number,
            content: line.content.clone(),
        }
    }
}

/// A stretch of the file the diff did not print, for a reader opening a gap.
///
/// The lines and nothing else: which line each one is, and which it was before
/// the change, the page works out from the hunks it already has.
#[derive(Debug, Serialize)]
pub struct LinesView {
    pub lines: Vec<String>,
}
