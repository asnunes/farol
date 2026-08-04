//! Text for the one consumer these commands have: the skill.
//!
//! Not JSON. The model has to reason about the map — is this block still true,
//! does this note still hold — and prose buried in escaped strings fights that
//! for no gain. Labelled, indented text reads the way the model needs it.
//!
//! Each report borrows what it prints and implements `Display`, so callers hand
//! it straight to `print!` and nothing builds a `String` it does not need.

use std::fmt::{self, Display, Write as _};

use crate::diff::domain::Scope;
use crate::map::application::CheckReport;
use crate::map::domain::{Orphan, ReviewMap, WORKING};

/// The files under review, as farol resolved them.
pub struct ScopeReport<'a>(pub &'a Scope);

impl Display for ScopeReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope = self.0;
        let separator = if scope.merge_base { "..." } else { ".." };
        writeln!(
            f,
            "review scope for {}{separator}{}",
            scope.base_ref, scope.head_ref
        )?;
        if scope.dirty {
            writeln!(f, "including uncommitted changes")?;
        }
        writeln!(f, "{} files\n", scope.files.len())?;

        for file in &scope.files {
            let rename = file
                .old_path
                .as_ref()
                .map(|p| format!(" (was {p})"))
                .unwrap_or_default();
            writeln!(
                f,
                "  {:<10} +{:<5} -{:<5} {}{rename}",
                file.status.label(),
                file.additions,
                file.deletions,
                file.path,
            )?;
        }
        Ok(())
    }
}

/// The map itself. Deliberately without the deactivated notes: this stays a
/// picture of the map rather than a picture of the map plus a work queue.
pub struct MapReport<'a> {
    pub map: &'a ReviewMap,
    pub behind: u32,
}

impl Display for MapReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let map = self.map;
        writeln!(f, "map for {} → {}", map.branch, map.base)?;

        let at = if map.generated_at == WORKING {
            "uncommitted work".to_string()
        } else {
            short(&map.generated_at)
        };
        match self.behind {
            0 => writeln!(f, "generated at {at}")?,
            1 => writeln!(f, "generated at {at} (1 commit behind HEAD)")?,
            n => writeln!(f, "generated at {at} ({n} commits behind HEAD)")?,
        }
        if let Some(parent) = &map.parent {
            writeln!(f, "derived from {}", short(parent))?;
        }

        if map.is_empty() {
            return writeln!(f, "\nThe map is empty — nothing has been mapped yet.");
        }

        for (i, block) in map.blocks.iter().enumerate() {
            writeln!(f, "\nblock {}  {}  \"{}\"", i + 1, block.slug, block.title)?;
            if !block.context.trim().is_empty() {
                writeln!(f, "  context: {}", indent_rest(&block.context, 4))?;
            }
            if !block.files.is_empty() {
                writeln!(f, "  files:")?;
                for file in &block.files {
                    writeln!(f, "    {}", file.path)?;
                    if let Some(note) = &file.note {
                        writeln!(f, "      note: {}", indent_rest(note, 8))?;
                    }
                    for note in &file.line_notes {
                        writeln!(
                            f,
                            "      lines {}: {}",
                            note.range,
                            indent_rest(&note.text, 8)
                        )?;
                    }
                }
            }
            let attached: Vec<_> = map.skim_for(&block.slug).collect();
            if !attached.is_empty() {
                writeln!(f, "  skim:")?;
                for entry in attached {
                    writeln!(f, "    {} — {}", entry.path, entry.reason)?;
                }
            }
        }

        let loose: Vec<_> = map.loose_skim().collect();
        if !loose.is_empty() {
            writeln!(f, "\nunassigned skim:")?;
            for entry in loose {
                writeln!(f, "  {} — {}", entry.path, entry.reason)?;
            }
        }
        Ok(())
    }
}

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

/// What the self-test found.
pub struct CheckSummary<'a>(pub &'a CheckReport);

impl Display for CheckSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let report = self.0;

        if !report.uncovered.is_empty() {
            writeln!(
                f,
                "{} file(s) in the review are in no block and not marked skim:",
                report.uncovered.len()
            )?;
            for path in &report.uncovered {
                writeln!(f, "  {path}")?;
            }
            writeln!(
                f,
                "\nA file nobody assigned never appears on screen. Put each one in a\nblock, in the `outros` block, or mark it skim."
            )?;
        }

        if report.pending_orphans > 0 {
            if !report.uncovered.is_empty() {
                writeln!(f)?;
            }
            writeln!(
                f,
                "{} deactivated line note(s) still undecided. Run `farol map derive`\nto list them, then restore or discard each one.",
                report.pending_orphans
            )?;
        }

        if report.passed() {
            writeln!(f, "Map is complete.")?;
            match report.commits_behind {
                0 => {}
                1 => writeln!(f, "Note: the map is 1 commit behind HEAD.")?,
                n => writeln!(f, "Note: the map is {n} commits behind HEAD.")?,
            }
        }
        Ok(())
    }
}

fn short(sha: &str) -> String {
    if sha == WORKING {
        sha.to_string()
    } else {
        sha.chars().take(7).collect()
    }
}

/// Keep wrapped prose lined up under its label instead of falling back to
/// column zero, where it would read as a new field.
fn indent_rest(text: &str, spaces: usize) -> String {
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
    use crate::map::domain::{LineRange, Position, ReviewMap};

    fn sample() -> ReviewMap {
        let mut m = ReviewMap::new("fix/bull-signing-handoff", "main", "a3f1e9c1234567");
        m.add_block(
            "recover-link",
            "Recover the Bull link before signing",
            "The first version created generic handoff abstractions.",
            Position::End,
        )
        .unwrap();
        m.add_file("recover-link", "services/bull_acceptance.go", None, None)
            .unwrap();
        m.add_line_note(
            "recover-link",
            "services/bull_acceptance.go",
            LineRange::new(82, 116).unwrap(),
            "Recovery only happens on a fresh transition.",
        )
        .unwrap();
        m.add_file(
            "recover-link",
            "services/controller.go",
            Some("The deletions are not an additional change.".into()),
            None,
        )
        .unwrap();
        m.add_skim("go.sum", "regenerated by the dependency change", None)
            .unwrap();
        m
    }

    fn render(map: &ReviewMap, behind: u32) -> String {
        MapReport { map, behind }.to_string()
    }

    #[test]
    fn the_map_renders_blocks_files_and_both_note_levels() {
        let out = render(&sample(), 2);
        assert!(out.contains("map for fix/bull-signing-handoff → main"));
        assert!(out.contains("generated at a3f1e9c (2 commits behind HEAD)"));
        assert!(out.contains("block 1  recover-link  \"Recover the Bull link before signing\""));
        assert!(out.contains("lines 82-116: Recovery only happens"));
        assert!(out.contains("note: The deletions are not an additional change."));
        assert!(out.contains("unassigned skim:"));
        assert!(out.contains("go.sum — regenerated by the dependency change"));
    }

    #[test]
    fn a_single_commit_behind_is_not_pluralised() {
        assert!(render(&sample(), 1).contains("(1 commit behind HEAD)"));
    }

    #[test]
    fn an_up_to_date_map_says_nothing_about_distance() {
        assert!(!render(&sample(), 0).contains("behind HEAD"));
    }

    #[test]
    fn skim_attached_to_a_block_renders_under_it_not_at_the_bottom() {
        let mut m = sample();
        m.add_skim(
            "controller_test.go",
            "only feeds the existing test",
            Some("recover-link".into()),
        )
        .unwrap();
        let out = render(&m, 0);
        assert!(
            out.contains("  skim:\n    controller_test.go"),
            "expected the attached skim inside the block:\n{out}"
        );
        assert!(out.contains("unassigned skim:\n  go.sum"));
    }

    #[test]
    fn an_empty_map_says_so_instead_of_printing_nothing() {
        let m = ReviewMap::new("b", "main", "abc1234");
        assert!(render(&m, 0).contains("The map is empty"));
    }

    #[test]
    fn orphans_print_both_commands_and_the_snapshot() {
        use crate::map::domain::{Orphan, OrphanReason};
        let orphans = vec![Orphan {
            block: "recover-link".into(),
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

    #[test]
    fn check_reports_uncovered_files_and_explains_why_it_matters() {
        let report = CheckReport {
            uncovered: vec!["forgotten.rs".into()],
            ..Default::default()
        };
        let out = CheckSummary(&report).to_string();
        assert!(out.contains("forgotten.rs"));
        assert!(out.contains("never appears on screen"));
        assert!(!out.contains("Map is complete"));
    }

    #[test]
    fn a_clean_check_says_so() {
        assert!(
            CheckSummary(&CheckReport::default())
                .to_string()
                .contains("Map is complete.")
        );
    }
}
