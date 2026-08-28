use crate::diff::application::ReviewScope;
use crate::diff::domain::Scope;
use crate::error::Result;
use crate::map::application::MapVersions;
use crate::map::domain::ReviewMap;
use crate::progress::application::ProgressStore;
use crate::progress::domain::Progress;

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

    /// Read afresh every time, never from something the caller is holding.
    ///
    /// The server used to keep the map it started with and hand that back on
    /// every request, so a map written while it ran reached the browser only
    /// after a restart. The watcher would announce the change and the answer
    /// would be the same as before.
    pub fn execute(&self) -> Result<ReviewSnapshot> {
        let map = self.versions.require_current()?;

        Ok(ReviewSnapshot {
            commits_behind: self.versions.behind(&map),
            map,
            scope: self.scope.get()?.clone(),
            progress: self.progress.load()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::MapError;
    use crate::progress::application::MarkViewed;
    use crate::testing::{FakeDiffSource, InMemoryProgressRepository, slug, use_case_setup};
    use std::sync::Arc;

    fn on(paths: &[&str]) -> (GetReview, ProgressStore, crate::map::application::MapEditor) {
        let (svc, scope) = use_case_setup(paths);
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
        editor
            .edit(|map| {
                map.add_block(
                    &slug("core"),
                    "The change",
                    "why",
                    crate::map::domain::Position::End,
                )
            })
            .unwrap();

        let snapshot = review.execute().unwrap();

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
        editor.edit(|_| Ok::<_, MapError>(())).unwrap();
        MarkViewed::new(progress).execute("a.rs", "now").unwrap();

        let snapshot = review.execute().unwrap();

        assert!(snapshot.progress.is_current("a.rs", "hash-of-a.rs"));
        assert!(!snapshot.progress.is_current("b.rs", "hash-of-b.rs"));
    }

    #[test]
    fn a_map_written_after_the_first_call_is_the_one_that_comes_back() {
        // The server keeps this use case for the life of the process and
        // answers every request with it. Holding on to the map it first read is
        // what left a freshly derived map invisible until a restart, while the
        // watcher announced a change that never arrived.
        let (review, _, editor) = on(&["a.rs"]);
        editor.edit(|_| Ok::<_, MapError>(())).unwrap();
        review.execute().unwrap();

        editor
            .edit(|map| {
                map.add_block(
                    &slug("later"),
                    "Written while the server ran",
                    "why",
                    crate::map::domain::Position::End,
                )
            })
            .unwrap();

        assert!(
            review
                .execute()
                .unwrap()
                .map
                .block(&slug("later"))
                .is_some()
        );
    }
}
