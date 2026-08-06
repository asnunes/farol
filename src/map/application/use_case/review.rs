//! Reading the map, and the version lifecycle around it.

use crate::diff::application::{FileDiffs, ReviewScope};
use crate::diff::domain::{ReviewPath, Scope};
use crate::map::application::{Derived, MapDerivation, MapEditor, MapVersions, ResetOutcome};
use crate::map::domain::ReviewMap;
use crate::progress::application::ProgressStore;
use crate::progress::domain::Progress;
use crate::shared::error::Result;

/// Produce the map version for the current commit, inheriting the last one.
/// Safe to call twice — the second call finds what the first made.
#[derive(Clone)]
pub struct DeriveMap {
    derivation: MapDerivation,
    /// Deriving reports how stale the version it inherited from was, which is
    /// a question about lineage rather than about deriving.
    versions: MapVersions,
}

impl DeriveMap {
    pub fn new(derivation: MapDerivation, versions: MapVersions) -> Self {
        Self {
            derivation,
            versions,
        }
    }

    pub fn execute(&self) -> Result<Derived> {
        self.derivation.derive()
    }

    /// How far `HEAD` has moved past the commit the map was built against.
    pub fn behind(&self, map: &ReviewMap) -> u32 {
        self.versions.behind(map)
    }
}

/// The map that belongs to where we are now, if there is one.
#[derive(Clone)]
pub struct ShowMap {
    versions: MapVersions,
}

impl ShowMap {
    pub fn new(versions: MapVersions) -> Self {
        Self { versions }
    }

    pub fn execute(&self) -> Result<Option<ReviewMap>> {
        self.versions.current()
    }

    /// The same, but saying so instead of returning nothing — what `serve` and
    /// `check` need, since neither has anything to do without a map.
    pub fn require(&self) -> Result<ReviewMap> {
        self.versions.require_current()
    }

    pub fn behind(&self, map: &ReviewMap) -> u32 {
        self.versions.behind(map)
    }
}

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
            pending_orphans: map.orphans.len(),
            commits_behind: self.versions.behind(&map),
        })
    }
}

/// Throw away the newest version so the one before it takes over.
#[derive(Clone)]
pub struct ResetMap {
    editor: MapEditor,
}

impl ResetMap {
    pub fn new(editor: MapEditor) -> Self {
        Self { editor }
    }

    pub fn execute(&self) -> Result<ResetOutcome> {
        self.editor.reset()
    }
}

/// The files under review, as farol resolved them.
#[derive(Clone)]
pub struct GetScope {
    scope: ReviewScope,
}

impl GetScope {
    pub fn new(scope: ReviewScope) -> Self {
        Self { scope }
    }

    pub fn execute(&self) -> Result<Scope> {
        self.scope.get().cloned()
    }

    /// Turning a raw path into a proven one is part of reading the scope, so it
    /// lives with it rather than in every use case that needs a path.
    pub fn path(&self, raw: &str) -> Result<ReviewPath> {
        self.scope.path(raw)
    }

    pub fn paths(&self, raw: &[String]) -> Result<Vec<ReviewPath>> {
        self.scope.paths(raw)
    }
}

/// Everything the screen needs in one call: the map, how far behind it is, and
/// what has been read.
#[derive(Clone)]
pub struct GetReview {
    versions: MapVersions,
    scope: ReviewScope,
    progress: ProgressStore,
}

pub struct ReviewSnapshot {
    pub map: ReviewMap,
    pub scope: Scope,
    pub progress: Progress,
    pub commits_behind: u32,
}

impl GetReview {
    pub fn new(versions: MapVersions, scope: ReviewScope, progress: ProgressStore) -> Self {
        Self {
            versions,
            scope,
            progress,
        }
    }

    pub fn execute(&self, map: &ReviewMap) -> Result<ReviewSnapshot> {
        Ok(ReviewSnapshot {
            map: map.clone(),
            scope: self.scope.get()?.clone(),
            progress: self.progress.load()?,
            commits_behind: self.versions.behind(map),
        })
    }
}

/// One file's diff, for the pane on the right.
#[derive(Clone)]
pub struct GetFileDiff {
    diffs: FileDiffs,
}

impl GetFileDiff {
    pub fn new(diffs: FileDiffs) -> Self {
        Self { diffs }
    }

    pub fn execute(&self, path: &str) -> Result<crate::diff::domain::FileDiff> {
        self.diffs.of(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::Position;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, slug};
    use std::sync::Arc;

    #[test]
    fn check_fails_on_a_file_nobody_assigned() {
        let source = FakeDiffSource::with_paths(&["a.rs", "forgotten.rs"]).on_commit("head");
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&[
            "a.rs",
            "forgotten.rs",
        ])));
        let maps = crate::testing::services(source, Arc::new(InMemoryMapRepository::new()));
        maps.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)
            })
            .unwrap();

        let report = CheckMap::new(maps.versions, scope).execute().unwrap();
        assert_eq!(report.uncovered, vec!["forgotten.rs"]);
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
}
