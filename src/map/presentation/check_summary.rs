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
}
