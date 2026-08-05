use std::sync::Arc;

use crate::diff::application::DiffService;
use crate::progress::domain::{Progress, ProgressRepository};
use crate::shared::error::Result;

/// Reading progress, over whatever storage and diff source it is given.
///
/// Both collaborators arrive from outside: the service never picks them, which
/// is what lets a test drive it with an in-memory repository and a hand-built
/// diff.
#[derive(Clone)]
pub struct ProgressService {
    repo: Arc<dyn ProgressRepository>,
    diff: DiffService,
}

impl ProgressService {
    pub fn new(repo: Arc<dyn ProgressRepository>, diff: DiffService) -> Self {
        Self { repo, diff }
    }

    pub fn load(&self) -> Result<Progress> {
        self.repo.load()
    }

    /// Mark a file read, pinned to the content it has right now.
    pub fn mark(&self, path: &str, at: &str) -> Result<Progress> {
        let diff = self.diff.file_diff(path)?;
        let mut progress = self.repo.load()?;
        progress.mark(path, diff.new_content_hash, at);
        self.repo.save(&progress)?;
        Ok(progress)
    }

    pub fn unmark(&self, path: &str) -> Result<Progress> {
        let mut progress = self.repo.load()?;
        progress.unmark(path);
        self.repo.save(&progress)?;
        Ok(progress)
    }
}
