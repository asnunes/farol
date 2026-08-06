use crate::map::application::MapVersions;
use crate::map::domain::ReviewMap;
use crate::shared::error::Result;

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
