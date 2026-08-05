//! Reading the map, and the version lifecycle around it.

use crate::diff::application::{FileDiffs, ReviewScope};
use crate::diff::domain::{ReviewPath, Scope};
use crate::map::application::{CheckReport, Derived, MapService, ResetOutcome};
use crate::map::domain::ReviewMap;
use crate::progress::application::ProgressStore;
use crate::progress::domain::Progress;
use crate::shared::error::Result;

/// Produce the map version for the current commit, inheriting the last one.
/// Safe to call twice — the second call finds what the first made.
#[derive(Clone)]
pub struct DeriveMap {
    maps: MapService,
}

impl DeriveMap {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self) -> Result<Derived> {
        self.maps.derive()
    }

    /// How far `HEAD` has moved past the commit the map was built against.
    pub fn behind(&self, map: &ReviewMap) -> u32 {
        self.maps.behind(map)
    }
}

/// The map that belongs to where we are now, if there is one.
#[derive(Clone)]
pub struct ShowMap {
    maps: MapService,
}

impl ShowMap {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self) -> Result<Option<ReviewMap>> {
        self.maps.current()
    }

    /// The same, but saying so instead of returning nothing — what `serve` and
    /// `check` need, since neither has anything to do without a map.
    pub fn require(&self) -> Result<ReviewMap> {
        self.maps.require_current()
    }

    pub fn behind(&self, map: &ReviewMap) -> u32 {
        self.maps.behind(map)
    }
}

/// Verify the map covers the review and leaves nothing undecided.
#[derive(Clone)]
pub struct CheckMap {
    maps: MapService,
}

impl CheckMap {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self) -> Result<CheckReport> {
        self.maps.check(&self.maps.require_current()?)
    }
}

/// Throw away the newest version so the one before it takes over.
#[derive(Clone)]
pub struct ResetMap {
    maps: MapService,
}

impl ResetMap {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self) -> Result<ResetOutcome> {
        self.maps.reset()
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
    maps: MapService,
    progress: ProgressStore,
}

pub struct ReviewSnapshot {
    pub map: ReviewMap,
    pub scope: Scope,
    pub progress: Progress,
    pub commits_behind: u32,
}

impl GetReview {
    pub fn new(maps: MapService, progress: ProgressStore) -> Self {
        Self { maps, progress }
    }

    pub fn execute(&self, map: &ReviewMap) -> Result<ReviewSnapshot> {
        Ok(ReviewSnapshot {
            map: map.clone(),
            scope: self.maps.scope()?.clone(),
            progress: self.progress.load()?,
            commits_behind: self.maps.behind(map),
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
