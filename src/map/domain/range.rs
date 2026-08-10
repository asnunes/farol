//! A span of lines, and what happens to it when the file moves underneath.
//!
//! Three outcomes, cheapest first:
//!
//! 1. Nothing above the span moved — it stays put.
//! 2. Lines moved above it but the covered code is untouched — it slides by the
//!    net delta. This is arithmetic over the diff, not text matching; it is what
//!    git itself does when applying a patch, and it absorbs most of the churn.
//! 3. The change reached inside the span — the note is deactivated. Guessing
//!    here would put a confident note on unrelated code, which is worse than
//!    showing nothing.

use serde::{Deserialize, Serialize};

use crate::diff::domain::{Hunk, ReviewPath};
use crate::shared::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LineRange {
    pub from: u32,
    pub to: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftOutcome {
    /// Still valid as written.
    Unchanged,
    /// Moved; the covered code is the same.
    Shifted(LineRange),
    /// The covered code itself changed. Only the session can decide.
    Overlapped,
}

impl LineRange {
    pub fn new(from: u32, to: u32) -> Result<Self> {
        if from == 0 || to < from {
            return Err(Error::BadRange {
                raw: format!("{from}-{to}"),
            });
        }
        Ok(Self { from, to })
    }

    /// Parse the `<from>-<to>` form the CLI takes.
    pub fn parse(raw: &str) -> Result<Self> {
        let bad = || Error::BadRange {
            raw: raw.to_string(),
        };
        let (a, b) = raw.split_once('-').ok_or_else(bad)?;
        let from = a.trim().parse::<u32>().map_err(|_| bad())?;
        let to = b.trim().parse::<u32>().map_err(|_| bad())?;
        Self::new(from, to)
    }

    /// Reject a span that points past the end of the file, where it would
    /// render nowhere. Takes a proven path because that is what carries the
    /// file's length — no reaching back for the diff source.
    pub fn require_within(&self, path: &ReviewPath) -> Result<()> {
        if self.to > path.lines() {
            return Err(Error::RangeOutOfFile {
                path: path.to_string(),
                from: self.from,
                to: self.to,
                total: path.lines(),
            });
        }
        Ok(())
    }

    /// Recompute against the hunks of a diff taken from the commit this span
    /// was written against to the commit being derived.
    pub fn shift(&self, hunks: &[Hunk]) -> ShiftOutcome {
        let mut delta: i64 = 0;

        for hunk in hunks {
            if self.overlaps(hunk) {
                return ShiftOutcome::Overlapped;
            }
            if self.sits_below(hunk) {
                delta += hunk.delta();
            }
        }

        if delta == 0 {
            return ShiftOutcome::Unchanged;
        }

        let from = self.from as i64 + delta;
        let to = self.to as i64 + delta;

        // Unreachable with well-formed hunks: everything above this span can
        // delete at most the lines that are above it, which lands it on line 1
        // and no higher. Kept because clamping a note to a line it was never
        // about is the one outcome worse than deactivating it, and the cost of
        // the guard is a comparison.
        if from < 1 || to < 1 {
            return ShiftOutcome::Overlapped;
        }

        ShiftOutcome::Shifted(Self {
            from: from as u32,
            to: to as u32,
        })
    }

    /// A pure insertion (`old_lines == 0`) replaces nothing, so it can never
    /// overlap — it only pushes things down.
    fn overlaps(&self, hunk: &Hunk) -> bool {
        if hunk.old_lines == 0 {
            return false;
        }
        let hunk_end = hunk.old_start + hunk.old_lines; // exclusive
        hunk.old_start <= self.to && self.from < hunk_end
    }

    fn sits_below(&self, hunk: &Hunk) -> bool {
        if hunk.old_lines == 0 {
            // Inserted after `old_start`, so it lifts this span only if it
            // lands strictly before it.
            hunk.old_start < self.from
        } else {
            hunk.old_start + hunk.old_lines <= self.from
        }
    }
}

impl std::fmt::Display for LineRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hunk(old_start: u32, old_lines: u32, new_lines: u32) -> Hunk {
        Hunk {
            old_start,
            old_lines,
            new_start: old_start,
            new_lines,
            lines: vec![],
        }
    }

    fn range() -> LineRange {
        LineRange::new(82, 116).unwrap()
    }

    #[test]
    fn ranges_parse_and_reject_nonsense() {
        assert_eq!(LineRange::parse("82-116").unwrap(), range());
        assert_eq!(LineRange::parse(" 5 - 9 ").unwrap().from, 5);
        assert!(LineRange::parse("82").is_err());
        assert!(LineRange::parse("0-9").is_err());
        assert!(LineRange::parse("9-5").is_err());
        assert!(LineRange::parse("a-b").is_err());
    }

    #[test]
    fn a_range_past_the_end_of_the_file_is_refused() {
        use crate::diff::domain::ReviewPath;
        assert!(
            range()
                .require_within(&ReviewPath::proven("a.rs", 200))
                .is_ok()
        );
        assert!(
            range()
                .require_within(&ReviewPath::proven("a.rs", 90))
                .is_err()
        );
    }

    #[test]
    fn no_hunks_leaves_the_range_alone() {
        assert_eq!(range().shift(&[]), ShiftOutcome::Unchanged);
    }

    #[test]
    fn change_below_the_note_does_not_move_it() {
        assert_eq!(range().shift(&[hunk(200, 4, 9)]), ShiftOutcome::Unchanged);
    }

    #[test]
    fn growth_above_pushes_the_note_down() {
        // 4 old lines became 9: five lines added above.
        assert_eq!(
            range().shift(&[hunk(10, 4, 9)]),
            ShiftOutcome::Shifted(LineRange { from: 87, to: 121 })
        );
    }

    #[test]
    fn shrinkage_above_pulls_the_note_up() {
        assert_eq!(
            range().shift(&[hunk(10, 9, 4)]),
            ShiftOutcome::Shifted(LineRange { from: 77, to: 111 })
        );
    }

    #[test]
    fn hunk_ending_exactly_before_the_note_still_counts_as_above() {
        // Covers 79, 80, 81 — the note starts at 82.
        assert_eq!(
            range().shift(&[hunk(79, 3, 5)]),
            ShiftOutcome::Shifted(LineRange { from: 84, to: 118 })
        );
    }

    #[test]
    fn hunk_starting_on_the_first_line_of_the_note_overlaps() {
        assert_eq!(range().shift(&[hunk(82, 1, 3)]), ShiftOutcome::Overlapped);
    }

    #[test]
    fn hunk_touching_the_last_line_of_the_note_overlaps() {
        assert_eq!(range().shift(&[hunk(116, 2, 2)]), ShiftOutcome::Overlapped);
    }

    #[test]
    fn hunk_starting_right_after_the_note_does_not_overlap() {
        assert_eq!(range().shift(&[hunk(117, 2, 6)]), ShiftOutcome::Unchanged);
    }

    #[test]
    fn hunk_swallowing_the_whole_note_overlaps() {
        assert_eq!(range().shift(&[hunk(70, 60, 12)]), ShiftOutcome::Overlapped);
    }

    #[test]
    fn pure_insertion_above_shifts_without_overlapping() {
        assert_eq!(
            range().shift(&[hunk(40, 0, 7)]),
            ShiftOutcome::Shifted(LineRange { from: 89, to: 123 })
        );
    }

    #[test]
    fn pure_insertion_inside_the_range_shifts_rather_than_orphans() {
        // Nothing the note described was rewritten, so the prose still holds —
        // the block just got longer. Deactivating here would be over-eager.
        assert_eq!(range().shift(&[hunk(90, 0, 4)]), ShiftOutcome::Unchanged);
    }

    #[test]
    fn several_hunks_above_accumulate() {
        // +4, -7, +1 = -2
        let hunks = vec![hunk(5, 2, 6), hunk(20, 10, 3), hunk(40, 1, 2)];
        assert_eq!(
            range().shift(&hunks),
            ShiftOutcome::Shifted(LineRange { from: 80, to: 114 })
        );
    }

    #[test]
    fn overlap_wins_over_any_shifting() {
        let hunks = vec![hunk(5, 2, 40), hunk(100, 3, 3)];
        assert_eq!(range().shift(&hunks), ShiftOutcome::Overlapped);
    }

    #[test]
    fn deletion_larger_than_the_offset_orphans_instead_of_clamping() {
        assert_eq!(range().shift(&[hunk(1, 90, 0)]), ShiftOutcome::Overlapped);
    }

    #[test]
    fn single_line_note_behaves_like_a_range_of_one() {
        let one = LineRange::new(50, 50).unwrap();
        assert_eq!(
            one.shift(&[hunk(10, 1, 4)]),
            ShiftOutcome::Shifted(LineRange { from: 53, to: 53 })
        );
    }
}
