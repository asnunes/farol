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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every reason, so a new variant cannot be added without deciding what to
    /// print and what to tell the reader to do about it.
    const ALL: [OrphanReason; 4] = [
        OrphanReason::HunkOverlap,
        OrphanReason::FileRenamed,
        OrphanReason::FileRemoved,
        OrphanReason::BlockRemoved,
    ];

    #[test]
    fn every_reason_has_a_label_the_map_file_can_carry() {
        for reason in ALL {
            let label = reason.label();
            assert!(!label.is_empty());
            assert_eq!(
                label,
                label.to_lowercase(),
                "labels are written into json and read back; keep them stable"
            );
        }
    }

    #[test]
    fn no_two_reasons_share_a_label() {
        let mut labels: Vec<&str> = ALL.iter().map(|r| r.label()).collect();
        labels.sort();
        let before = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), before);
    }

    #[test]
    fn every_reason_tells_the_reader_what_to_do_about_it() {
        // The audience is the session writing the map. A reason without a way
        // out leaves it guessing between restore and discard.
        for reason in ALL {
            assert!(!reason.guidance().is_empty(), "{:?}", reason);
        }
    }

    #[test]
    fn a_file_that_is_gone_is_told_to_discard_rather_than_restore() {
        // There is nowhere to restore it to, and suggesting otherwise would
        // send the session looking for a path that does not exist.
        assert!(OrphanReason::FileRemoved.guidance().contains("discard"));
        assert!(
            !OrphanReason::FileRemoved
                .guidance()
                .contains("restore against")
        );
    }

    #[test]
    fn a_label_survives_the_trip_through_json() {
        // The map file is read back by a later run; a renamed variant would
        // silently discard every orphan written before it.
        let json = serde_json::to_string(&OrphanReason::HunkOverlap).unwrap();
        assert_eq!(json, "\"hunk-overlap\"");
        let back: OrphanReason = serde_json::from_str(&json).unwrap();
        assert_eq!(back, OrphanReason::HunkOverlap);
    }
}
