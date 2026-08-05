//! Reading progress: the store that keeps it, and the two use cases over it.

use std::sync::Arc;

use crate::diff::application::DiffService;
use crate::progress::domain::{Progress, ProgressRepository};
use crate::shared::error::Result;

/// A **service**: the dependency the use cases share. Not called by transports.
#[derive(Clone)]
pub struct ProgressStore {
    repo: Arc<dyn ProgressRepository>,
    diff: DiffService,
}

impl ProgressStore {
    pub fn new(repo: Arc<dyn ProgressRepository>, diff: DiffService) -> Self {
        Self { repo, diff }
    }

    pub fn load(&self) -> Result<Progress> {
        self.repo.load()
    }

    fn save(&self, progress: &Progress) -> Result<()> {
        self.repo.save(progress)
    }

    fn content_hash(&self, path: &str) -> Result<String> {
        Ok(self.diff.file_diff(path)?.new_content_hash)
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
