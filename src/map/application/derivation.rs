//! Producing the version for the commit we are standing on.
//!
//! Deriving rather than regenerating exists for one reason: whoever is reading
//! may be halfway through. Rebuilding from scratch reshuffles block boundaries
//! and names, and they lose their place even though the new map is just as
//! good. A typo commit must not cost that.

use std::sync::Arc;

use super::reconciler::MapReconciler;
use super::versions::MapVersions;
use crate::diff::application::ReviewScope;
use crate::error::Result;
use crate::map::domain::{MapRepository, ReviewMap, WORKING};

#[derive(Clone)]
pub struct MapDerivation {
    versions: MapVersions,
    scope: ReviewScope,
    /// What to do to an inherited map when the code underneath it moved.
    reconciler: MapReconciler,
    repo: Arc<dyn MapRepository>,
}

impl MapDerivation {
    pub fn new(
        versions: MapVersions,
        scope: ReviewScope,
        reconciler: MapReconciler,
        repo: Arc<dyn MapRepository>,
    ) -> Self {
        Self {
            versions,
            scope,
            reconciler,
            repo,
        }
    }

    /// Idempotent: creates the version for the current commit if it is missing,
    /// returns the existing one otherwise. Either way the caller can read the
    /// pending orphans off the map, so a run that died halfway simply picks up.
    pub fn derive(&self) -> Result<Derived> {
        let target = self.versions.target()?;

        if let Some(existing) = self.versions.at_target()? {
            return Ok(Derived {
                map: existing,
                created: false,
            });
        }

        let scope = self.scope.get()?;
        let mut map = match self.versions.parent_of(&target)? {
            Some((parent_sha, parent_map)) => {
                let mut m = parent_map;
                m.parent = Some(parent_sha.clone());
                m.generated_at = target.clone();
                m.branch = scope.branch.clone();
                m.base = scope.base_ref.clone();
                // Orphans belong to the version that produced them; the next one
                // starts clean. Carrying them forever would pile up a graveyard
                // nobody revisits.
                m.clear_orphans();
                self.reconciler.reanchor(&mut m, &parent_sha, &target)?;
                m
            }
            None => ReviewMap::new(&scope.branch, &scope.base_ref, &target),
        };

        self.reconciler.prune_gone_files(&mut map)?;
        self.repo.save(&map)?;

        // A working map that has been absorbed must not keep being picked as
        // the parent of every future commit.
        if !scope.dirty && map.parent.as_deref() == Some(WORKING) {
            self.repo.delete(WORKING)?;
        }

        Ok(Derived { map, created: true })
    }
}

pub struct Derived {
    pub map: ReviewMap,
    /// True when this call created the version rather than finding it.
    pub created: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::MapError;
    use crate::map::domain::{LineRange, NoteFate, OrphanReason, Position};
    use crate::testing::{
        FakeDiffSource, InMemoryMapRepository, hunk, map_with_note, services, slug,
    };
    use std::sync::Arc;

    #[test]
    fn deriving_twice_on_the_same_commit_returns_the_existing_version() {
        let source = FakeDiffSource::with_paths(&["a.rs"]).on_commit("head");
        let svc = services(source, Arc::new(InMemoryMapRepository::new()));

        assert!(svc.derivation.derive().unwrap().created);
        assert!(
            !svc.derivation.derive().unwrap().created,
            "a second call must find the version, not make another"
        );
    }

    #[test]
    fn deriving_runs_the_reconciler_over_the_inherited_map() {
        // Derivation's own job is to invoke it with the right pair of commits;
        // what the reconciler then does is its own tests' business.
        let source = FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("new")
            .with_ancestors(&["old"])
            .at_distance("old", 1)
            .changed_between("old", "new", "a.rs", vec![hunk(1, 0, 5)]);

        let repo = Arc::new(InMemoryMapRepository::new());
        repo.seed(map_with_note(
            "old",
            LineRange::new(40, 45).unwrap(),
            "still true",
        ));

        let map = services(source, repo).derivation.derive().unwrap().map;
        let notes = &map
            .block(&slug("core"))
            .unwrap()
            .file("a.rs")
            .unwrap()
            .line_notes;
        assert_eq!(
            notes[0].range,
            LineRange::new(45, 50).unwrap(),
            "the note should have been moved, which only happens if the \
             reconciler ran"
        );
    }

    #[test]
    fn the_inherited_map_points_back_at_what_it_came_from() {
        let source = FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("new")
            .with_ancestors(&["old"])
            .at_distance("old", 1);
        let repo = Arc::new(InMemoryMapRepository::new());
        repo.seed(map_with_note("old", LineRange::new(1, 2).unwrap(), "x"));

        let map = services(source, repo).derivation.derive().unwrap().map;

        assert_eq!(map.generated_at, "new");
        assert_eq!(map.parent.as_deref(), Some("old"));
    }

    #[test]
    fn orphans_do_not_survive_into_the_next_version() {
        let source = FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("newer")
            .with_ancestors(&["new"])
            .at_distance("new", 1);

        let repo = Arc::new(InMemoryMapRepository::new());
        let mut stale = map_with_note("new", LineRange::new(1, 2).unwrap(), "x");
        // Deactivate it the way a derivation would, so the next one inherits a
        // map with an undecided orphan on it.
        stale
            .reanchor_notes(|_, _| {
                Ok::<_, MapError>(NoteFate::Orphan {
                    snapshot: String::new(),
                    reason: OrphanReason::HunkOverlap,
                })
            })
            .unwrap();
        repo.seed(stale);

        let map = services(source, repo).derivation.derive().unwrap().map;
        assert!(
            map.orphans().is_empty(),
            "carrying them forever would pile up a graveyard nobody revisits"
        );
    }

    #[test]
    fn a_first_map_on_a_branch_with_no_ancestor_starts_empty() {
        let source = FakeDiffSource::with_paths(&["a.rs"]).on_commit("head");

        let map = services(source, Arc::new(InMemoryMapRepository::new()))
            .derivation
            .derive()
            .unwrap()
            .map;

        assert!(map.blocks().is_empty());
        assert_eq!(map.generated_at, "head");
        assert_eq!(map.parent, None);
    }

    #[test]
    fn committing_absorbs_the_working_map_instead_of_leaving_it_behind() {
        // The map written against uncommitted work becomes the map of the
        // commit. Leaving the working copy in place would make it the parent
        // of every future commit, so the branch would keep inheriting from a
        // version that no longer means anything.
        let repo = Arc::new(InMemoryMapRepository::new());
        let dirty = services(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .dirty(),
            repo.clone(),
        );
        dirty
            .editor
            .edit(|m| m.add_block(&slug("core"), "written while dirty", "c", Position::End))
            .unwrap();
        assert_eq!(repo.stored_shas().unwrap(), vec![WORKING]);

        // Now it is committed: the same work, under a sha.
        let committed = services(
            FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"),
            repo.clone(),
        );
        let derived = committed.derivation.derive().unwrap();

        assert!(derived.created);
        assert_eq!(derived.map.generated_at, "head");
        assert_eq!(
            derived.map.block(&slug("core")).unwrap().title,
            "written while dirty",
            "the prose has to survive the commit"
        );
        assert_eq!(
            repo.stored_shas().unwrap(),
            vec!["head"],
            "the working copy is gone, not merely superseded"
        );
    }

    #[test]
    fn a_working_map_is_kept_while_the_work_is_still_uncommitted() {
        // Deriving again while dirty must not delete what it just inherited.
        let repo = Arc::new(InMemoryMapRepository::new());
        let svc = services(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .dirty(),
            repo.clone(),
        );
        svc.editor.edit(|_| Ok::<_, MapError>(())).unwrap();

        svc.derivation.derive().unwrap();

        assert_eq!(repo.stored_shas().unwrap(), vec![WORKING]);
    }
}
