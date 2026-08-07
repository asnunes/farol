//! A note the code moved out from under, and why.

use serde::{Deserialize, Serialize};

use super::range::LineRange;
use super::slug::Slug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrphanReason {
    /// The code under the note was rewritten.
    HunkOverlap,
    FileRenamed,
    FileRemoved,
    BlockRemoved,
}

impl OrphanReason {
    pub fn label(&self) -> &'static str {
        match self {
            OrphanReason::HunkOverlap => "hunk-overlap",
            OrphanReason::FileRenamed => "file-renamed",
            OrphanReason::FileRemoved => "file-removed",
            OrphanReason::BlockRemoved => "block-removed",
        }
    }

    pub fn guidance(&self) -> &'static str {
        match self {
            OrphanReason::HunkOverlap => {
                "re-read the new code; restore with the new range if the note still holds, otherwise discard"
            }
            OrphanReason::FileRenamed => "restore against the new path",
            OrphanReason::FileRemoved => "discard — there is nowhere to restore it",
            OrphanReason::BlockRemoved => "discard, or restore into another block",
        }
    }
}

/// A line note whose anchor stopped being trustworthy. The prose is kept — it
/// was the expensive part — along with the code it used to cover, which is what
/// actually identifies it. The old range is only a hint about where to look.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Orphan {
    pub block: Slug,
    pub path: String,
    #[serde(flatten)]
    pub old_range: LineRange,
    pub snapshot: String,
    pub reason: OrphanReason,
    pub text: String,
}
