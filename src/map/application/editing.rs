//! Changing the map that applies now, and throwing a version away.
//!
//! Every write goes through here, so the load-change-store sequence exists in
//! one place: a use case says what to change, not how to persist it.

use std::sync::Arc;

use super::derivation::MapDerivation;
use super::versions::MapVersions;
#[cfg(test)]
use crate::map::domain::Position;
use crate::map::domain::{MapRepository, ReviewMap};
use crate::shared::error::Result;

pub enum ResetOutcome {
    /// Deleted; the named version is current again, or nothing is.
    Deleted {
        fell_back_to: Option<String>,
    },
    NothingToDelete,
}

#[derive(Clone)]
pub struct MapEditor {
    /// An edit derives first, so writing to a commit that has no version yet
    /// starts one rather than failing.
    derivation: MapDerivation,
    versions: MapVersions,
    repo: Arc<dyn MapRepository>,
}

impl MapEditor {
    pub fn new(
        derivation: MapDerivation,
        versions: MapVersions,
        repo: Arc<dyn MapRepository>,
    ) -> Self {
        Self {
            derivation,
            versions,
            repo,
        }
    }

    /// Load the version for where we are, apply `f`, store it back.
    pub fn edit<F>(&self, f: F) -> Result<ReviewMap>
    where
        F: FnOnce(&mut ReviewMap) -> Result<()>,
    {
        let mut map = self.derivation.derive()?.map;
        f(&mut map)?;
        self.repo.save(&map)?;
        Ok(map)
    }

    /// Drop the version for where we are, so the previous one applies again.
    pub fn reset(&self) -> Result<ResetOutcome> {
        let target = self.versions.target()?;
        if self.versions.at_target()?.is_none() {
            return Ok(ResetOutcome::NothingToDelete);
        }
        self.repo.delete(&target)?;
        Ok(ResetOutcome::Deleted {
            fell_back_to: self.versions.current()?.map(|m| m.generated_at),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services, slug};
    use std::sync::Arc;

    fn on(source: FakeDiffSource) -> crate::testing::MapServices {
        services(source, Arc::new(InMemoryMapRepository::new()))
    }

    fn head() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"]).on_commit("head")
    }

    #[test]
    fn an_edit_on_a_commit_with_no_version_yet_starts_one() {
        // Otherwise the first `block add` on a new commit would fail and the
        // author would have to know to run `map derive` first.
        let svc = on(head());

        let map = svc
            .editor
            .edit(|m| m.add_block(&slug("core"), "t", "c", Position::End))
            .unwrap();

        assert_eq!(map.generated_at, "head");
        assert!(map.block(&slug("core")).is_some());
    }

    #[test]
    fn an_edit_is_stored_so_the_next_command_sees_it() {
        let svc = on(head());
        svc.editor
            .edit(|m| m.add_block(&slug("core"), "t", "c", Position::End))
            .unwrap();

        let stored = svc.versions.require_current().unwrap();

        assert!(stored.block(&slug("core")).is_some());
    }

    #[test]
    fn an_edit_that_fails_stores_nothing() {
        // Half a change is worse than none: the author would have to work out
        // which part of their command landed.
        let svc = on(head());
        svc.editor
            .edit(|m| m.add_block(&slug("core"), "t", "c", Position::End))
            .unwrap();

        let err = svc.editor.edit(|m| {
            m.add_block(&slug("second"), "t", "c", Position::End)?;
            m.add_block(&slug("core"), "duplicate", "c", Position::End)
        });

        assert!(err.is_err());
        assert_eq!(
            svc.versions.require_current().unwrap().slugs(),
            vec!["core"],
            "the block added before the failure must not have been stored"
        );
    }

    #[test]
    fn resetting_with_no_version_here_says_there_was_nothing_to_drop() {
        let svc = on(head());

        assert!(matches!(
            svc.editor.reset().unwrap(),
            ResetOutcome::NothingToDelete
        ));
    }

    #[test]
    fn resetting_drops_this_version_and_names_what_takes_over() {
        let repo = Arc::new(InMemoryMapRepository::new());
        repo.seed(ReviewMap::new("feature/x", "main", "old"));
        let svc = services(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .with_ancestors(&["old"])
                .at_distance("old", 1),
            repo,
        );
        svc.editor.edit(|_| Ok(())).unwrap(); // a version for "head"

        let outcome = svc.editor.reset().unwrap();

        match outcome {
            ResetOutcome::Deleted { fell_back_to } => {
                assert_eq!(fell_back_to.as_deref(), Some("old"))
            }
            ResetOutcome::NothingToDelete => panic!("there was a version to drop"),
        }
        assert_eq!(
            svc.versions.current().unwrap().unwrap().generated_at,
            "old",
            "the ancestor applies again"
        );
    }

    #[test]
    fn resetting_the_only_version_leaves_nothing_current() {
        let svc = on(head());
        svc.editor.edit(|_| Ok(())).unwrap();

        let outcome = svc.editor.reset().unwrap();

        match outcome {
            ResetOutcome::Deleted { fell_back_to } => assert_eq!(fell_back_to, None),
            ResetOutcome::NothingToDelete => panic!("there was a version to drop"),
        }
        assert!(svc.versions.current().unwrap().is_none());
    }
}
