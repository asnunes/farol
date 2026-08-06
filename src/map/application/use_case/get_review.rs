use crate::diff::application::ReviewScope;
use crate::diff::domain::Scope;
use crate::map::application::MapVersions;
use crate::map::domain::ReviewMap;
use crate::progress::application::ProgressStore;
use crate::progress::domain::Progress;
use crate::shared::error::Result;

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
