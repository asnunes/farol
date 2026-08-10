//! Notes the code moved out from under, and what to do about each.

use std::fmt::{self, Display};

use super::text::indent_rest;
use crate::map::domain::Orphan;

/// Deactivated notes, printed by `map derive` and nowhere else.
pub struct OrphanReport<'a>(pub &'a [Orphan]);

impl Display for OrphanReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        let plural = if self.0.len() == 1 { "note" } else { "notes" };
        writeln!(
            f,
            "\n{} line {plural} deactivated — decide each one before finishing.",
            self.0.len()
        )?;
        writeln!(
            f,
            "Find where the code went using the snapshot, not the old line numbers."
        )?;

        for orphan in self.0 {
            writeln!(f, "\n  {} in block {}", orphan.path, orphan.block)?;
            writeln!(
                f,
                "    was at {} · {} — {}",
                orphan.old_range,
                orphan.reason.label(),
                orphan.reason.guidance()
            )?;
            writeln!(f, "    note: {}", indent_rest(&orphan.text, 10))?;
            if !orphan.snapshot.trim().is_empty() {
                writeln!(f, "    code it covered:")?;
                for line in orphan.snapshot.lines().take(12) {
                    writeln!(f, "      {line}")?;
                }
            }
            // Printing the exact commands spares the skill from deducing the
            // syntax, which is one more thing it could get subtly wrong.
            writeln!(
                f,
                "    restore: farol line restore {} {} {} --range <new-range>",
                orphan.block, orphan.path, orphan.old_range
            )?;
            writeln!(
                f,
                "    discard: farol line discard {} {} {}",
                orphan.block, orphan.path, orphan.old_range
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::LineRange;
    use crate::testing::slug;

    #[test]
    fn orphans_print_both_commands_and_the_snapshot() {
        use crate::map::domain::{Orphan, OrphanReason};
        let orphans = vec![Orphan {
            block: slug("retry-window"),
            path: "src/retry.rs".into(),
            old_range: LineRange::new(82, 116).unwrap(),
            snapshot: "if attempt.state == State::Pending {".into(),
            reason: OrphanReason::HunkOverlap,
            text: "The backoff resets only on a fresh attempt.".into(),
        }];
        let out = OrphanReport(&orphans).to_string();
        assert!(out.contains("1 line note deactivated"));
        assert!(out.contains("hunk-overlap"));
        assert!(out.contains("if attempt.state == State::Pending {"));
        assert!(
            out.contains("farol line restore retry-window src/retry.rs 82-116 --range <new-range>")
        );
        assert!(out.contains("farol line discard retry-window src/retry.rs 82-116"));
    }

    #[test]
    fn no_orphans_prints_nothing_at_all() {
        assert_eq!(OrphanReport(&[]).to_string(), "");
    }

    fn orphan(range: LineRange, text: &str) -> crate::map::domain::Orphan {
        use crate::map::domain::{Orphan, OrphanReason};
        Orphan {
            block: slug("retry-window"),
            path: "src/retry.rs".into(),
            old_range: range,
            snapshot: "if attempt.state == State::Pending {".into(),
            reason: OrphanReason::HunkOverlap,
            text: text.into(),
        }
    }

    #[test]
    fn one_note_is_not_reported_in_the_plural() {
        let out = OrphanReport(&[orphan(LineRange::new(1, 2).unwrap(), "one")]).to_string();

        assert!(out.contains("1 line note deactivated"), "{out}");
        assert!(!out.contains("notes deactivated"), "{out}");
    }

    #[test]
    fn several_notes_are() {
        let out = OrphanReport(&[
            orphan(LineRange::new(1, 2).unwrap(), "one"),
            orphan(LineRange::new(9, 9).unwrap(), "two"),
        ])
        .to_string();

        assert!(out.contains("2 line notes deactivated"), "{out}");
    }

    #[test]
    fn the_reader_is_told_to_go_by_the_snapshot_not_the_old_numbers() {
        // After a refactor the old range may point at a different function
        // entirely, and restoring there would place a confident note on
        // unrelated code.
        let out = OrphanReport(&[orphan(LineRange::new(82, 116).unwrap(), "n")]).to_string();

        assert!(out.contains("not the old line numbers"), "{out}");
    }

    #[test]
    fn every_orphan_carries_the_reason_and_what_to_do_about_it() {
        let out = OrphanReport(&[orphan(LineRange::new(1, 2).unwrap(), "n")]).to_string();

        assert!(out.contains("hunk-overlap"), "{out}");
        assert!(out.contains("re-read the new code"), "{out}");
    }
}
