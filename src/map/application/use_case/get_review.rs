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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::application::MarkViewed;
    use crate::testing::{
        FakeDiffSource, InMemoryMapRepository, InMemoryProgressRepository, services, slug,
    };
    use std::sync::Arc;

    fn on(paths: &[&str]) -> (GetReview, ProgressStore, crate::map::application::MapEditor) {
        let svc = services(
            FakeDiffSource::with_paths(paths).on_commit("head"),
            Arc::new(InMemoryMapRepository::new()),
        );
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(paths)));
        let progress = ProgressStore::new(
            Arc::new(InMemoryProgressRepository::default()),
            crate::diff::application::FileDiffs::new(Arc::new(FakeDiffSource::with_paths(paths))),
        );
        (
            GetReview::new(svc.versions, scope, progress.clone()),
            progress,
            svc.editor,
        )
    }

    #[test]
    fn one_call_carries_everything_the_screen_needs() {
        let (review, _, editor) = on(&["a.rs", "b.rs"]);
        let map = editor
            .edit(|map| {
                map.add_block(
                    &slug("core"),
                    "The change",
                    "why",
                    crate::map::domain::Position::End,
                )
            })
            .unwrap();

        let snapshot = review.execute(&map).unwrap();

        assert_eq!(
            snapshot.map.block(&slug("core")).unwrap().title,
            "The change"
        );
        assert_eq!(snapshot.scope.files.len(), 2, "the window, for the sidebar");
        assert_eq!(snapshot.commits_behind, 0);
        assert!(snapshot.progress.viewed.is_empty());
    }

    #[test]
    fn what_the_reviewer_has_read_comes_back_with_it() {
        // The screen strikes files through on load; without this the reviewer
        // would start over every refresh.
        let (review, progress, editor) = on(&["a.rs", "b.rs"]);
        let map = editor.edit(|_| Ok(())).unwrap();
        MarkViewed::new(progress).execute("a.rs", "now").unwrap();

        let snapshot = review.execute(&map).unwrap();

        assert!(snapshot.progress.is_current("a.rs", "hash-of-a.rs"));
        assert!(!snapshot.progress.is_current("b.rs", "hash-of-b.rs"));
    }

    #[test]
    fn a_map_from_an_earlier_commit_reports_how_far_behind_it_is() {
        let (review, _, _) = on(&["a.rs"]);
        let map = crate::map::domain::ReviewMap::new("feature/x", "main", "old");

        // The fake places no distance on "old", so this is the shape of the
        // answer rather than the arithmetic, which `MapVersions` owns.
        assert_eq!(review.execute(&map).unwrap().commits_behind, 0);
    }
}
