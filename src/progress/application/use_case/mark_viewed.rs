use crate::error::Result;
use crate::progress::application::ProgressStore;
use crate::progress::domain::Progress;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::application::store_over;
    use crate::progress::domain::ProgressRepository;
    use crate::testing::FakeDiffSource;

    #[test]
    fn marking_a_file_pins_it_to_the_content_it_had_when_it_was_read() {
        let (store, repo) = store_over(FakeDiffSource::with_paths(&["a.rs"]));

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
        let (store, _) = store_over(FakeDiffSource::with_paths(&["a.rs"]));
        let progress = MarkViewed::new(store).execute("a.rs", "abc123").unwrap();

        assert!(!progress.is_current("a.rs", "hash-after-the-fix"));
    }

    #[test]
    fn marking_a_file_that_is_not_under_review_fails_before_anything_is_written() {
        let (store, repo) = store_over(FakeDiffSource::with_paths(&["a.rs"]));

        assert!(
            MarkViewed::new(store)
                .execute("elsewhere.rs", "abc")
                .is_err()
        );
        assert!(repo.load().unwrap().viewed.is_empty());
    }
}
