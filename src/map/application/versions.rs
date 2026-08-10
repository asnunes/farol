//! Which stored version applies to where we are, and how far behind it is.
//!
//! Read-only. It changes when the rules of lineage change — what counts as an
//! ancestor, how uncommitted work is keyed — and for no other reason.

use std::sync::Arc;

use crate::diff::application::{CommitHistory, ReviewScope};
use crate::error::Result;
use crate::map::domain::{MapError, MapRepository, ReviewMap, WORKING};

#[derive(Clone)]
pub struct MapVersions {
    scope: ReviewScope,
    history: CommitHistory,
    repo: Arc<dyn MapRepository>,
}

impl MapVersions {
    pub fn new(scope: ReviewScope, history: CommitHistory, repo: Arc<dyn MapRepository>) -> Self {
        Self {
            scope,
            history,
            repo,
        }
    }

    /// The version key for where we are: a commit, or the working-tree marker.
    pub fn target(&self) -> Result<String> {
        let scope = self.scope.get()?;
        Ok(if scope.dirty {
            WORKING.to_string()
        } else {
            scope.head_sha.clone()
        })
    }

    /// The version stored for exactly where we are, if there is one.
    pub fn at_target(&self) -> Result<Option<ReviewMap>> {
        self.repo.load_at(&self.target()?)
    }

    /// The map that belongs to where we are now, falling back to the newest
    /// ancestor that has one — which is what makes "two commits behind" a state
    /// rather than an absence.
    pub fn current(&self) -> Result<Option<ReviewMap>> {
        if let Some(map) = self.at_target()? {
            return Ok(Some(map));
        }
        Ok(self.nearest_ancestor(&self.target()?)?.map(|(_, map)| map))
    }

    /// Same as `current`, but says so instead of returning nothing.
    pub fn require_current(&self) -> Result<ReviewMap> {
        self.current()?.ok_or_else(|| {
            MapError::NoMap {
                branch: self
                    .scope
                    .get()
                    .map(|s| s.branch.clone())
                    .unwrap_or_default(),
            }
            .into()
        })
    }

    /// How far `HEAD` has moved past the commit this map was built against.
    pub fn behind(&self, map: &ReviewMap) -> u32 {
        self.history.commits_ahead_of(&map.generated_at)
    }

    /// The map to inherit from: uncommitted work first, then the newest stored
    /// commit that is an ancestor of where we are now.
    pub fn parent_of(&self, target: &str) -> Result<Option<(String, ReviewMap)>> {
        if target != WORKING
            && let Some(wip) = self.repo.load_at(WORKING)?
        {
            return Ok(Some((WORKING.to_string(), wip)));
        }
        self.nearest_ancestor(target)
    }

    fn nearest_ancestor(&self, target: &str) -> Result<Option<(String, ReviewMap)>> {
        let mut best: Option<(u32, String)> = None;
        for sha in self.repo.stored_shas()? {
            if sha == target || sha == WORKING || !self.history.is_ancestor(&sha)? {
                continue;
            }
            let distance = self.history.commits_ahead_of(&sha);
            if best.as_ref().map(|(d, _)| distance < *d).unwrap_or(true) {
                best = Some((distance, sha));
            }
        }
        match best {
            Some((_, sha)) => Ok(self.repo.load_at(&sha)?.map(|m| (sha, m))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services};
    use std::sync::Arc;

    fn seeded(source: FakeDiffSource, shas: &[&str]) -> MapVersions {
        let repo = Arc::new(InMemoryMapRepository::new());
        for sha in shas {
            repo.seed(ReviewMap::new("feature/x", "main", *sha));
        }
        services(source, repo).versions
    }

    #[test]
    fn the_version_for_this_commit_wins_over_any_ancestor() {
        let versions = seeded(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .with_ancestors(&["old"])
                .at_distance("old", 3),
            &["head", "old"],
        );

        assert_eq!(versions.current().unwrap().unwrap().generated_at, "head");
    }

    #[test]
    fn with_nothing_here_the_nearest_ancestor_applies() {
        // This is what makes "two commits behind" a state rather than an
        // absence: the reviewer keeps reading the map they have.
        let versions = seeded(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .with_ancestors(&["near", "far"])
                .at_distance("near", 2)
                .at_distance("far", 9),
            &["near", "far"],
        );

        let map = versions.current().unwrap().unwrap();
        assert_eq!(map.generated_at, "near", "the closest one, not just any");
        assert_eq!(versions.behind(&map), 2);
    }

    #[test]
    fn a_version_from_a_commit_that_is_not_an_ancestor_is_ignored() {
        // Another branch's map describes code that is not on this one.
        let versions = seeded(
            FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"),
            &["elsewhere"],
        );

        assert!(versions.current().unwrap().is_none());
    }

    #[test]
    fn asking_for_a_map_that_is_not_there_names_the_branch() {
        let versions = seeded(FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"), &[]);

        let err = versions.require_current().unwrap_err();
        assert!(err.to_string().contains("feature/x"), "{err}");
    }

    #[test]
    fn uncommitted_work_is_keyed_apart_from_the_commit_it_sits_on() {
        let versions = seeded(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .dirty(),
            &[],
        );

        assert_eq!(versions.target().unwrap(), WORKING);
    }

    #[test]
    fn a_working_map_is_inherited_ahead_of_any_commit() {
        // Work in progress is the newest thing there is, whatever the history
        // says about distances.
        let source = FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("head")
            .with_ancestors(&["old"])
            .at_distance("old", 1);
        let repo = Arc::new(InMemoryMapRepository::new());
        repo.seed(ReviewMap::new("feature/x", "main", "old"));
        repo.seed(ReviewMap::new("feature/x", "main", WORKING));
        let versions = services(source, repo).versions;

        let (sha, _) = versions.parent_of("head").unwrap().unwrap();
        assert_eq!(sha, WORKING);
    }
}
