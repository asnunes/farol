//! Where a line note goes when the file changes underneath it.
//!
//! Three outcomes, cheapest first:
//!
//! 1. Nothing above the note moved — the note stays put.
//! 2. Lines moved above it but the covered code is untouched — the range slides
//!    by the net delta. This is arithmetic over the diff, not text matching; it
//!    is what git itself does when applying a patch, and it absorbs most of the
//!    churn.
//! 3. The change reached inside the covered range — the note is deactivated.
//!    Guessing here would put a confident note on unrelated code, which is
//!    worse than showing nothing.

use crate::diff::domain::Hunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftOutcome {
    /// Range is still valid as written.
    Unchanged,
    /// Range moved; the covered code is the same.
    Shifted { from: u32, to: u32 },
    /// The covered code itself changed. Only a human or the session can decide.
    Overlapped,
}

/// Recompute `from..=to` against the hunks of a diff taken from the commit the
/// note was written against to the commit being derived.
pub fn shift_range(from: u32, to: u32, hunks: &[Hunk]) -> ShiftOutcome {
    let mut delta: i64 = 0;

    for hunk in hunks {
        if overlaps(hunk, from, to) {
            return ShiftOutcome::Overlapped;
        }
        if entirely_above(hunk, from) {
            delta += hunk.delta();
        }
    }

    if delta == 0 {
        return ShiftOutcome::Unchanged;
    }

    // A hunk above deleting more than the note's own offset would push it off
    // the top of the file. Treat that as overlap rather than clamping to a
    // line that means nothing.
    let new_from = from as i64 + delta;
    let new_to = to as i64 + delta;
    if new_from < 1 || new_to < 1 {
        return ShiftOutcome::Overlapped;
    }

    ShiftOutcome::Shifted {
        from: new_from as u32,
        to: new_to as u32,
    }
}

/// A pure insertion (`old_lines == 0`) replaces nothing, so it can never
/// overlap — it only pushes things down.
fn overlaps(hunk: &Hunk, from: u32, to: u32) -> bool {
    if hunk.old_lines == 0 {
        return false;
    }
    let hunk_end = hunk.old_start + hunk.old_lines; // exclusive
    hunk.old_start <= to && from < hunk_end
}

fn entirely_above(hunk: &Hunk, from: u32) -> bool {
    if hunk.old_lines == 0 {
        // Inserted after `old_start`, so it lifts the note only if it lands
        // strictly before it.
        hunk.old_start < from
    } else {
        hunk.old_start + hunk.old_lines <= from
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::domain::Hunk;

    /// Hunks carry no line bodies here — shifting only reads the counts.
    fn hunk(old_start: u32, old_lines: u32, new_lines: u32) -> Hunk {
        Hunk {
            old_start,
            old_lines,
            new_start: old_start,
            new_lines,
            lines: vec![],
        }
    }

    #[test]
    fn no_hunks_leaves_the_range_alone() {
        assert_eq!(shift_range(82, 116, &[]), ShiftOutcome::Unchanged);
    }

    #[test]
    fn change_below_the_note_does_not_move_it() {
        let hunks = vec![hunk(200, 4, 9)];
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Unchanged);
    }

    #[test]
    fn growth_above_pushes_the_note_down() {
        // 4 old lines became 9: five lines added above the note.
        let hunks = vec![hunk(10, 4, 9)];
        assert_eq!(
            shift_range(82, 116, &hunks),
            ShiftOutcome::Shifted { from: 87, to: 121 }
        );
    }

    #[test]
    fn shrinkage_above_pulls_the_note_up() {
        let hunks = vec![hunk(10, 9, 4)];
        assert_eq!(
            shift_range(82, 116, &hunks),
            ShiftOutcome::Shifted { from: 77, to: 111 }
        );
    }

    #[test]
    fn hunk_ending_exactly_before_the_note_still_counts_as_above() {
        // Covers 79, 80, 81 — the note starts at 82.
        let hunks = vec![hunk(79, 3, 5)];
        assert_eq!(
            shift_range(82, 116, &hunks),
            ShiftOutcome::Shifted { from: 84, to: 118 }
        );
    }

    #[test]
    fn hunk_starting_on_the_first_line_of_the_note_overlaps() {
        let hunks = vec![hunk(82, 1, 3)];
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Overlapped);
    }

    #[test]
    fn hunk_touching_the_last_line_of_the_note_overlaps() {
        let hunks = vec![hunk(116, 2, 2)];
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Overlapped);
    }

    #[test]
    fn hunk_starting_right_after_the_note_does_not_overlap() {
        let hunks = vec![hunk(117, 2, 6)];
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Unchanged);
    }

    #[test]
    fn hunk_swallowing_the_whole_note_overlaps() {
        let hunks = vec![hunk(70, 60, 12)];
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Overlapped);
    }

    #[test]
    fn pure_insertion_above_shifts_without_overlapping() {
        let hunks = vec![hunk(40, 0, 7)];
        assert_eq!(
            shift_range(82, 116, &hunks),
            ShiftOutcome::Shifted { from: 89, to: 123 }
        );
    }

    #[test]
    fn pure_insertion_inside_the_range_shifts_rather_than_orphans() {
        // Nothing the note described was rewritten, so the prose still holds —
        // the block just got longer. Deactivating here would be over-eager.
        let hunks = vec![hunk(90, 0, 4)];
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Unchanged);
    }

    #[test]
    fn several_hunks_above_accumulate() {
        let hunks = vec![hunk(5, 2, 6), hunk(20, 10, 3), hunk(40, 1, 2)];
        // +4, -7, +1 = -2
        assert_eq!(
            shift_range(82, 116, &hunks),
            ShiftOutcome::Shifted { from: 80, to: 114 }
        );
    }

    #[test]
    fn overlap_wins_over_any_shifting() {
        let hunks = vec![hunk(5, 2, 40), hunk(100, 3, 3)];
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Overlapped);
    }

    #[test]
    fn deletion_larger_than_the_offset_orphans_instead_of_clamping() {
        let hunks = vec![hunk(1, 90, 0)];
        // The note's own lines are inside that deletion anyway.
        assert_eq!(shift_range(82, 116, &hunks), ShiftOutcome::Overlapped);
    }

    #[test]
    fn single_line_note_behaves_like_a_range_of_one() {
        let hunks = vec![hunk(10, 1, 4)];
        assert_eq!(
            shift_range(50, 50, &hunks),
            ShiftOutcome::Shifted { from: 53, to: 53 }
        );
    }
}
