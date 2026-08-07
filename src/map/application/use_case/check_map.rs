use crate::diff::application::ReviewScope;
use crate::map::application::MapVersions;
use crate::shared::error::Result;

/// What the self-test found.
#[derive(Debug, Default)]
pub struct CheckReport {
    pub uncovered: Vec<String>,
    pub pending_orphans: usize,
    pub commits_behind: u32,
}

impl CheckReport {
    pub fn passed(&self) -> bool {
        self.uncovered.is_empty() && self.pending_orphans == 0
    }
}

/// Verify the map covers the review and leaves nothing undecided.
///
/// It exists so "every file shows up somewhere" does not depend on the model
/// remembering the rule: a file nobody assigned is not merely undocumented, it
/// is invisible, because the sidebar is built from the map.
#[derive(Clone)]
pub struct CheckMap {
    versions: MapVersions,
    scope: ReviewScope,
}

impl CheckMap {
    pub fn new(versions: MapVersions, scope: ReviewScope) -> Self {
        Self { versions, scope }
    }

    pub fn execute(&self) -> Result<CheckReport> {
        let map = self.versions.require_current()?;
        let covered = map.covered_paths();
        Ok(CheckReport {
            uncovered: self
                .scope
                .get()?
                .files
                .iter()
                .map(|f| f.path.clone())
                .filter(|p| !covered.contains(p))
                .collect(),
            pending_orphans: map.orphans().len(),
            commits_behind: self.versions.behind(&map),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services, slug};
    use std::sync::Arc;

    #[test]
    fn check_fails_on_a_file_nobody_assigned() {
        let paths = ["a.rs", "forgotten.rs"];
        let svc = services(
            FakeDiffSource::with_paths(&paths).on_commit("head"),
            Arc::new(InMemoryMapRepository::new()),
        );
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        svc.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", crate::map::domain::Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)
            })
            .unwrap();

        let report = CheckMap::new(svc.versions, scope).execute().unwrap();

        assert_eq!(report.uncovered, vec!["forgotten.rs"]);
        assert!(!report.passed());
    }

    #[test]
    fn a_map_that_covers_every_file_and_leaves_no_orphan_passes() {
        let paths = ["a.rs", "b.rs"];
        let svc = services(
            FakeDiffSource::with_paths(&paths).on_commit("head"),
            Arc::new(InMemoryMapRepository::new()),
        );
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        svc.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", crate::map::domain::Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)?;
                map.add_file(&slug("core"), "b.rs", None, None)
            })
            .unwrap();

        let report = CheckMap::new(svc.versions, scope).execute().unwrap();

        assert!(report.uncovered.is_empty());
        assert_eq!(report.pending_orphans, 0);
        assert!(report.passed());
    }

    #[test]
    fn a_file_on_the_skim_list_counts_as_covered() {
        // Saying "read this diagonally" is a decision about the file, not a
        // failure to decide.
        let paths = ["a.rs", "Cargo.lock"];
        let svc = services(
            FakeDiffSource::with_paths(&paths).on_commit("head"),
            Arc::new(InMemoryMapRepository::new()),
        );
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        svc.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", crate::map::domain::Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)?;
                map.add_skim("Cargo.lock", "regenerated", None)
            })
            .unwrap();

        assert!(
            CheckMap::new(svc.versions, scope)
                .execute()
                .unwrap()
                .passed()
        );
    }

    #[test]
    fn an_orphan_nobody_decided_about_holds_the_check_open() {
        let svc = services(
            FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"),
            Arc::new(InMemoryMapRepository::new()),
        );
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&["a.rs"])));
        svc.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", crate::map::domain::Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)?;
                map.add_line_note(
                    &slug("core"),
                    "a.rs",
                    crate::testing::range(1, 2),
                    "undecided",
                )?;
                // Deactivated the way a derivation would leave it.
                map.reanchor_notes(|_, _| {
                    Ok(crate::map::domain::NoteFate::Orphan {
                        snapshot: String::new(),
                        reason: crate::map::domain::OrphanReason::HunkOverlap,
                    })
                })?;
                Ok(())
            })
            .unwrap();

        let report = CheckMap::new(svc.versions, scope).execute().unwrap();

        assert!(report.uncovered.is_empty());
        assert_eq!(report.pending_orphans, 1);
        assert!(!report.passed());
    }

    #[test]
    fn a_report_only_passes_when_nothing_is_left_open() {
        assert!(CheckReport::default().passed());
        assert!(
            !CheckReport {
                uncovered: vec!["a.rs".into()],
                ..Default::default()
            }
            .passed()
        );
        assert!(
            !CheckReport {
                pending_orphans: 1,
                ..Default::default()
            }
            .passed()
        );
    }

    #[test]
    fn being_behind_is_reported_but_does_not_fail_the_check() {
        // Staleness is something to know, not something that makes the map
        // wrong: the reviewer can keep reading while the branch moves.
        assert!(
            CheckReport {
                commits_behind: 4,
                ..Default::default()
            }
            .passed()
        );
    }
}
