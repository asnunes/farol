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
            block: slug("recover-link"),
            path: "a.go".into(),
            old_range: LineRange::new(82, 116).unwrap(),
            snapshot: "if offer.Status == StatusSigning {".into(),
            reason: OrphanReason::HunkOverlap,
            text: "Recovery only happens on a fresh transition.".into(),
        }];
        let out = OrphanReport(&orphans).to_string();
        assert!(out.contains("1 line note deactivated"));
        assert!(out.contains("hunk-overlap"));
        assert!(out.contains("if offer.Status == StatusSigning {"));
        assert!(out.contains("farol line restore recover-link a.go 82-116 --range <new-range>"));
        assert!(out.contains("farol line discard recover-link a.go 82-116"));
    }

    #[test]
    fn no_orphans_prints_nothing_at_all() {
        assert_eq!(OrphanReport(&[]).to_string(), "");
    }
}
