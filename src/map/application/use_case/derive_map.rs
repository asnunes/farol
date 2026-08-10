use crate::error::Result;
use crate::map::application::{Derived, MapDerivation, MapVersions};
use crate::map::domain::ReviewMap;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services};
    use std::sync::Arc;

    fn on(source: FakeDiffSource) -> DeriveMap {
        let svc = services(source, Arc::new(InMemoryMapRepository::new()));
        DeriveMap::new(svc.derivation, svc.versions)
    }

    #[test]
    fn the_first_call_on_a_branch_makes_the_version_for_this_commit() {
        let derive = on(FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"));

        let out = derive.execute().unwrap();

        assert!(out.created);
        assert_eq!(out.map.generated_at, "head");
        assert_eq!(out.map.branch, "feature/x");
        assert_eq!(out.map.base, "main");
    }

    #[test]
    fn a_second_call_finds_what_the_first_made() {
        // The skill can be interrupted and re-run; that must not fork the map.
        let derive = on(FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"));
        derive.execute().unwrap();

        let out = derive.execute().unwrap();

        assert!(!out.created);
        assert_eq!(out.map.generated_at, "head");
    }

    #[test]
    fn how_far_behind_a_map_is_comes_from_the_history() {
        let derive = on(FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("head")
            .at_distance("old", 4));
        let map = crate::map::domain::ReviewMap::new("feature/x", "main", "old");

        assert_eq!(derive.behind(&map), 4);
    }

    #[test]
    fn the_version_for_where_we_are_is_never_behind_itself() {
        let derive = on(FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"));

        let out = derive.execute().unwrap();

        assert_eq!(derive.behind(&out.map), 0);
    }
}
