//! Whether the map is finished, and what is still open if it is not.

use std::fmt::{self, Display};

use crate::map::application::CheckReport;

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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn every_unassigned_file_is_named_and_the_cost_is_spelled_out() {
        // The reader is the session that wrote the map. "3 files" without the
        // names leaves it grepping.
        let out = CheckSummary(&CheckReport {
            uncovered: vec!["src/a.rs".into(), "src/b.rs".into()],
            ..Default::default()
        })
        .to_string();

        assert!(out.contains("2 file(s)"), "{out}");
        assert!(
            out.contains("src/a.rs") && out.contains("src/b.rs"),
            "{out}"
        );
        assert!(
            out.contains("never appears on screen"),
            "the consequence is why this matters: {out}"
        );
    }

    #[test]
    fn undecided_orphans_are_counted_with_the_command_that_lists_them() {
        let out = CheckSummary(&CheckReport {
            pending_orphans: 3,
            ..Default::default()
        })
        .to_string();

        assert!(out.contains("3 deactivated"), "{out}");
        assert!(out.contains("farol map derive"), "{out}");
    }

    #[test]
    fn the_two_kinds_of_unfinished_business_are_kept_apart() {
        let out = CheckSummary(&CheckReport {
            uncovered: vec!["src/a.rs".into()],
            pending_orphans: 1,
            ..Default::default()
        })
        .to_string();

        let files_at = out.find("1 file(s)").expect("uncovered section");
        let orphans_at = out.find("1 deactivated").expect("orphan section");
        assert!(files_at < orphans_at);
        assert!(
            out[files_at..orphans_at].contains("\n\n"),
            "a blank line between them, or they read as one paragraph:\n{out}"
        );
    }

    #[test]
    fn a_complete_map_says_so_in_one_line() {
        let out = CheckSummary(&CheckReport::default()).to_string();

        assert!(out.contains("Map is complete."), "{out}");
        assert!(!out.contains("behind HEAD"), "{out}");
    }

    #[test]
    fn a_complete_map_that_is_behind_says_how_far_without_failing() {
        // Staleness is something to know, not something that makes the map
        // wrong.
        let one = CheckSummary(&CheckReport {
            commits_behind: 1,
            ..Default::default()
        })
        .to_string();
        assert!(one.contains("1 commit behind"), "{one}");
        assert!(!one.contains("1 commits"), "not pluralised: {one}");

        let many = CheckSummary(&CheckReport {
            commits_behind: 4,
            ..Default::default()
        })
        .to_string();
        assert!(many.contains("4 commits behind"), "{many}");
    }

    #[test]
    fn an_unfinished_map_is_not_told_how_stale_it_is() {
        // There is a more pressing thing to fix first.
        let out = CheckSummary(&CheckReport {
            uncovered: vec!["src/a.rs".into()],
            commits_behind: 4,
            ..Default::default()
        })
        .to_string();

        assert!(!out.contains("behind HEAD"), "{out}");
    }
}
