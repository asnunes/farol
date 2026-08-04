use crate::diff::domain::DiffSource;
use crate::progress::domain::{Progress, ProgressRepository};
use crate::shared::error::Result;

/// Reading progress, over whatever storage and diff source it is given.
///
/// Both collaborators arrive from outside: the service never picks them, which
/// is what lets a test drive it with an in-memory repository and a hand-built
/// diff.
pub struct ProgressService<'a> {
    repo: &'a dyn ProgressRepository,
    source: &'a dyn DiffSource,
}

impl<'a> ProgressService<'a> {
    pub fn new(repo: &'a dyn ProgressRepository, source: &'a dyn DiffSource) -> Self {
        Self { repo, source }
    }

    pub fn load(&self) -> Result<Progress> {
        self.repo.load()
    }

    /// Mark a file read, pinned to the content it has right now.
    pub fn mark(&self, path: &str, at: &str) -> Result<Progress> {
        let diff = self.source.file_diff(path)?;
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
