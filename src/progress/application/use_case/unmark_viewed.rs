use crate::error::Result;
use crate::progress::application::ProgressStore;
use crate::progress::domain::Progress;

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
    use crate::progress::application::MarkViewed;
    use crate::progress::application::store_over;
    use crate::progress::domain::ProgressRepository;
    use crate::testing::FakeDiffSource;

    #[test]
    fn unmarking_takes_the_tick_off_and_leaves_the_others_alone() {
        let (store, repo) = store_over(FakeDiffSource::with_paths(&["a.rs", "b.rs"]));
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
    fn unmarking_a_file_that_was_never_read_is_not_an_error() {
        // The browser can send this on a double click; refusing would turn a
        // harmless race into an error banner.
        let (store, _) = store_over(FakeDiffSource::with_paths(&["a.rs"]));

        assert!(UnmarkViewed::new(store).execute("a.rs").is_ok());
    }
}
