use crate::map::application::{Derived, MapDerivation, MapVersions};
use crate::map::domain::ReviewMap;
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
