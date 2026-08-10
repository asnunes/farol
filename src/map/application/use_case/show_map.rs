use crate::error::Result;
use crate::map::application::MapVersions;
use crate::map::domain::ReviewMap;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services, slug};
    use std::sync::Arc;

    fn on(source: FakeDiffSource) -> (ShowMap, crate::map::application::MapEditor) {
        let svc = services(source, Arc::new(InMemoryMapRepository::new()));
        (ShowMap::new(svc.versions), svc.editor)
    }

    #[test]
    fn the_map_that_was_written_is_the_map_that_comes_back() {
        let (show, editor) = on(FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"));
        editor
            .edit(|map| {
                map.add_block(
                    &slug("core"),
                    "The change itself",
                    "why",
                    crate::map::domain::Position::End,
                )
            })
            .unwrap();

        let map = show.execute().unwrap().expect("a map was written");

        assert_eq!(map.generated_at, "head");
        assert_eq!(map.block(&slug("core")).unwrap().title, "The change itself");
    }

    #[test]
    fn nothing_written_yet_is_nothing_to_show() {
        // `map show` on a fresh branch reports emptiness rather than failing;
        // it is the command you run to find out where you stand.
        let (show, _) = on(FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"));

        assert!(show.execute().unwrap().is_none());
    }

    #[test]
    fn requiring_a_map_that_is_not_there_names_the_branch() {
        // `serve` and `check` have nothing to do without one, so they say which
        // branch is missing it rather than showing an empty screen.
        let (show, _) = on(FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"));

        let err = show.require().unwrap_err();

        assert!(err.to_string().contains("feature/x"), "{err}");
    }

    #[test]
    fn with_nothing_here_the_ancestors_map_shows_and_says_how_stale_it_is() {
        // Three commits on and still reading the map you have: that is the
        // ordinary state, not a failure.
        let repo = Arc::new(InMemoryMapRepository::new());
        repo.seed(crate::map::domain::ReviewMap::new(
            "feature/x",
            "main",
            "old",
        ));
        let svc = services(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .with_ancestors(&["old"])
                .at_distance("old", 3),
            repo,
        );

        let show = ShowMap::new(svc.versions);
        let map = show.execute().unwrap().expect("the ancestor's map applies");

        assert_eq!(map.generated_at, "old");
        assert_eq!(show.behind(&map), 3);
    }
}
