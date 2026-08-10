//! Reading progress: the store that keeps it, and the two use cases over it.

use std::sync::Arc;

use crate::diff::application::FileDiffs;
use crate::error::Result;
use crate::progress::domain::{Progress, ProgressRepository};

/// A **service**: the dependency the use cases share. Not called by transports.
#[derive(Clone)]
pub struct ProgressStore {
    repo: Arc<dyn ProgressRepository>,
    /// Only for the content hash a tick is pinned to — this store has no
    /// business walking history or resolving scope.
    diffs: FileDiffs,
}

impl ProgressStore {
    pub fn new(repo: Arc<dyn ProgressRepository>, diffs: FileDiffs) -> Self {
        Self { repo, diffs }
    }

    pub fn load(&self) -> Result<Progress> {
        self.repo.load()
    }

    fn save(&self, progress: &Progress) -> Result<()> {
        self.repo.save(progress)
    }

    fn content_hash(&self, path: &str) -> Result<String> {
        self.diffs.content_hash(path)
    }
}

/// Record that a file has been read, pinned to the content it has right now —
/// which is what makes it reopen when the author changes it.
#[derive(Clone)]
pub struct MarkViewed {
    store: ProgressStore,
}

impl MarkViewed {
    pub fn new(store: ProgressStore) -> Self {
        Self { store }
    }

    pub fn execute(&self, path: &str, at: &str) -> Result<Progress> {
        let hash = self.store.content_hash(path)?;
        let mut progress = self.store.load()?;
        progress.mark(path, hash, at);
        self.store.save(&progress)?;
        Ok(progress)
    }
}

/// Take the tick back off a file.
#[derive(Clone)]
pub struct UnmarkViewed {
    store: ProgressStore,
}

impl UnmarkViewed {
    pub fn new(store: ProgressStore) -> Self {
        Self { store }
    }

    pub fn execute(&self, path: &str) -> Result<Progress> {
        let mut progress = self.store.load()?;
        progress.unmark(path);
        self.store.save(&progress)?;
        Ok(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDiffSource, InMemoryProgressRepository};

    fn store(source: FakeDiffSource) -> (ProgressStore, Arc<InMemoryProgressRepository>) {
        let repo = Arc::new(InMemoryProgressRepository::default());
        (
            ProgressStore::new(repo.clone(), FileDiffs::new(Arc::new(source))),
            repo,
        )
    }

    #[test]
    fn marking_a_file_pins_it_to_the_content_it_had_when_it_was_read() {
        let (store, repo) = store(FakeDiffSource::with_paths(&["a.rs"]));

        let progress = MarkViewed::new(store).execute("a.rs", "abc123").unwrap();

        assert!(progress.is_current("a.rs", "hash-of-a.rs"));
        assert!(
            repo.load().unwrap().is_current("a.rs", "hash-of-a.rs"),
            "a tick the server never wrote down is a tick the reviewer loses"
        );
    }

    #[test]
    fn a_file_whose_content_changed_since_it_was_read_is_no_longer_viewed() {
        // The pin is the whole mechanism: the author pushes a fix, and the file
        // reopens by itself instead of staying struck through.
        let (store, _) = store(FakeDiffSource::with_paths(&["a.rs"]));
        let progress = MarkViewed::new(store).execute("a.rs", "abc123").unwrap();

        assert!(!progress.is_current("a.rs", "hash-after-the-fix"));
    }

    #[test]
    fn unmarking_takes_the_tick_off_and_leaves_the_others_alone() {
        let (store, repo) = store(FakeDiffSource::with_paths(&["a.rs", "b.rs"]));
        MarkViewed::new(store.clone())
            .execute("a.rs", "abc")
            .unwrap();
        MarkViewed::new(store.clone())
            .execute("b.rs", "abc")
            .unwrap();

        let progress = UnmarkViewed::new(store).execute("a.rs").unwrap();

        assert!(!progress.is_current("a.rs", "hash-of-a.rs"));
        assert!(progress.is_current("b.rs", "hash-of-b.rs"));
        assert_eq!(repo.load().unwrap().viewed.len(), 1);
    }

    #[test]
    fn marking_a_file_that_is_not_under_review_fails_before_anything_is_written() {
        let (store, repo) = store(FakeDiffSource::with_paths(&["a.rs"]));

        assert!(
            MarkViewed::new(store)
                .execute("elsewhere.rs", "abc")
                .is_err()
        );
        assert!(repo.load().unwrap().viewed.is_empty());
    }

    #[test]
    fn unmarking_a_file_that_was_never_read_is_not_an_error() {
        // The browser can send this on a double click; refusing would turn a
        // harmless race into an error banner.
        let (store, _) = store(FakeDiffSource::with_paths(&["a.rs"]));

        assert!(UnmarkViewed::new(store).execute("a.rs").is_ok());
    }
}
