//! Text for the one consumer these commands have: the skill.
//!
//! Not JSON. The model has to reason about the map — is this block still true,
//! does this note still hold — and prose buried in escaped strings fights that
//! for no gain. Labelled, indented text reads the way the model needs it.

use std::fmt::Write;

use crate::diff::domain::Scope;
use crate::map::application::CheckReport;
use crate::map::domain::{Orphan, ReviewMap, WORKING};

pub fn render_scope(scope: &Scope) -> String {
    let mut out = String::new();
    let window = if scope.merge_base {
        format!("{}...{}", scope.base_ref, scope.head_ref)
    } else {
        format!("{}..{}", scope.base_ref, scope.head_ref)
    };
    let _ = writeln!(out, "review scope for {window}");
    if scope.dirty {
        let _ = writeln!(out, "including uncommitted changes");
    }
    let _ = writeln!(out, "{} files\n", scope.files.len());

    for f in &scope.files {
        let rename = f
            .old_path
            .as_ref()
            .map(|p| format!(" (was {p})"))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "  {:<10} +{:<5} -{:<5} {}{}",
            f.status.label(),
            f.additions,
            f.deletions,
            f.path,
            rename
        );
    }
    out
}

pub fn render_map(map: &ReviewMap, behind: u32) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "map for {} → {}", map.branch, map.base);

    let at = if map.generated_at == WORKING {
        "uncommitted work".to_string()
    } else {
        short(&map.generated_at)
    };
    if behind > 0 {
        let plural = if behind == 1 { "commit" } else { "commits" };
        let _ = writeln!(out, "generated at {at} ({behind} {plural} behind HEAD)");
    } else {
        let _ = writeln!(out, "generated at {at}");
    }
    if let Some(parent) = &map.parent {
        let _ = writeln!(out, "derived from {}", short(parent));
    }

    if map.is_empty() {
        let _ = writeln!(out, "\nThe map is empty — nothing has been mapped yet.");
        return out;
    }

    for (i, block) in map.blocks.iter().enumerate() {
        let _ = writeln!(
            out,
            "\nblock {}  {}  \"{}\"",
            i + 1,
            block.slug,
            block.title
        );
        if !block.context.trim().is_empty() {
            let _ = writeln!(out, "  context: {}", indent_rest(&block.context, 4));
        }
        if !block.files.is_empty() {
            let _ = writeln!(out, "  files:");
            for file in &block.files {
                let _ = writeln!(out, "    {}", file.path);
                if let Some(note) = &file.note {
                    let _ = writeln!(out, "      note: {}", indent_rest(note, 8));
                }
                for note in &file.line_notes {
                    let _ = writeln!(
                        out,
                        "      lines {}: {}",
                        note.range(),
                        indent_rest(&note.text, 8)
                    );
                }
            }
        }
        let attached: Vec<_> = map
            .skim
            .iter()
            .filter(|s| s.block.as_deref() == Some(block.slug.as_str()))
            .collect();
        if !attached.is_empty() {
            let _ = writeln!(out, "  skim:");
            for s in attached {
                let _ = writeln!(out, "    {} — {}", s.path, s.reason);
            }
        }
    }

    let loose: Vec<_> = map.skim.iter().filter(|s| s.block.is_none()).collect();
    if !loose.is_empty() {
        let _ = writeln!(out, "\nunassigned skim:");
        for s in loose {
            let _ = writeln!(out, "  {} — {}", s.path, s.reason);
        }
    }

    out
}

/// Deactivated notes, printed by `map derive` and nowhere else — `map show`
/// stays a picture of the map rather than a picture of the map plus a work
/// queue.
pub fn render_orphans(orphans: &[Orphan]) -> String {
    if orphans.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let plural = if orphans.len() == 1 { "note" } else { "notes" };
    let _ = writeln!(
        out,
        "\n{} line {plural} deactivated — decide each one before finishing.",
        orphans.len()
    );
    let _ = writeln!(
        out,
        "Find where the code went using the snapshot, not the old line numbers."
    );

    for o in orphans {
        let _ = writeln!(out, "\n  {} in block {}", o.path, o.block);
        let _ = writeln!(
            out,
            "    was at {} · {} — {}",
            o.old_range(),
            o.reason.label(),
            o.reason.guidance()
        );
        let _ = writeln!(out, "    note: {}", indent_rest(&o.text, 10));
        if !o.snapshot.trim().is_empty() {
            let _ = writeln!(out, "    code it covered:");
            for line in o.snapshot.lines().take(12) {
                let _ = writeln!(out, "      {line}");
            }
        }
        let _ = writeln!(
            out,
            "    restore: farol line restore {} {} {} --range <new-range>",
            o.block,
            o.path,
            o.old_range()
        );
        let _ = writeln!(
            out,
            "    discard: farol line discard {} {} {}",
            o.block,
            o.path,
            o.old_range()
        );
    }
    out
}

pub fn render_check(report: &CheckReport) -> String {
    let mut out = String::new();

    if !report.uncovered.is_empty() {
        let _ = writeln!(
            out,
            "{} file(s) in the review are in no block and not marked skim:",
            report.uncovered.len()
        );
        for p in &report.uncovered {
            let _ = writeln!(out, "  {p}");
        }
        let _ = writeln!(
            out,
            "\nA file nobody assigned never appears on screen. Put each one in a\nblock, in the `outros` block, or mark it skim."
        );
    }

    if report.pending_orphans > 0 {
        if !out.is_empty() {
            let _ = writeln!(out);
        }
        let _ = writeln!(
            out,
            "{} deactivated line note(s) still undecided. Run `farol map derive`\nto list them, then restore or discard each one.",
            report.pending_orphans
        );
    }

    if report.passed() {
        let _ = writeln!(out, "Map is complete.");
        if report.commits_behind > 0 {
            let plural = if report.commits_behind == 1 {
                "commit"
            } else {
                "commits"
            };
            let _ = writeln!(
                out,
                "Note: the map is {} {plural} behind HEAD.",
                report.commits_behind
            );
        }
    }

    out
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
    let pad = " ".repeat(spaces);
    text.trim().replace('\n', &format!("\n{pad}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::{Position, ReviewMap};

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
            82,
            116,
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

    #[test]
    fn the_map_renders_blocks_files_and_both_note_levels() {
        let out = render_map(&sample(), 2);
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
        assert!(render_map(&sample(), 1).contains("(1 commit behind HEAD)"));
    }

    #[test]
    fn an_up_to_date_map_says_nothing_about_distance() {
        assert!(!render_map(&sample(), 0).contains("behind HEAD"));
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
        let out = render_map(&m, 0);
        let under_block = out.find("  skim:\n    controller_test.go").is_some();
        assert!(
            under_block,
            "expected the attached skim inside the block:\n{out}"
        );
        assert!(out.contains("unassigned skim:\n  go.sum"));
    }

    #[test]
    fn an_empty_map_says_so_instead_of_printing_nothing() {
        let m = ReviewMap::new("b", "main", "abc1234");
        assert!(render_map(&m, 0).contains("The map is empty"));
    }

    #[test]
    fn orphans_print_both_commands_and_the_snapshot() {
        use crate::map::domain::{Orphan, OrphanReason};
        let orphans = vec![Orphan {
            block: "recover-link".into(),
            path: "a.go".into(),
            old_from: 82,
            old_to: 116,
            snapshot: "if offer.Status == StatusSigning {".into(),
            reason: OrphanReason::HunkOverlap,
            text: "Recovery only happens on a fresh transition.".into(),
        }];
        let out = render_orphans(&orphans);
        assert!(out.contains("1 line note deactivated"));
        assert!(out.contains("hunk-overlap"));
        assert!(out.contains("if offer.Status == StatusSigning {"));
        assert!(out.contains("farol line restore recover-link a.go 82-116 --range <new-range>"));
        assert!(out.contains("farol line discard recover-link a.go 82-116"));
    }

    #[test]
    fn no_orphans_prints_nothing_at_all() {
        assert_eq!(render_orphans(&[]), "");
    }

    #[test]
    fn check_reports_uncovered_files_and_explains_why_it_matters() {
        let report = CheckReport {
            uncovered: vec!["forgotten.rs".into()],
            pending_orphans: 0,
            commits_behind: 0,
        };
        let out = render_check(&report);
        assert!(out.contains("forgotten.rs"));
        assert!(out.contains("never appears on screen"));
        assert!(!out.contains("Map is complete"));
    }

    #[test]
    fn a_clean_check_says_so() {
        let out = render_check(&CheckReport::default());
        assert!(out.contains("Map is complete."));
    }
}
