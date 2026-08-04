use crate::diff::domain::DiffSource;
use crate::progress::domain::{Progress, ProgressRepository};
use crate::shared::error::Result;

/// Mark a file read, pinned to the content it has right now.
pub fn mark_viewed(
    repo: &dyn ProgressRepository,
    source: &dyn DiffSource,
    path: &str,
    at: &str,
) -> Result<Progress> {
    let diff = source.file_diff(path)?;
    let mut progress = repo.load()?;
    progress.mark(path, diff.new_content_hash, at);
    repo.save(&progress)?;
    Ok(progress)
}

pub fn unmark_viewed(repo: &dyn ProgressRepository, path: &str) -> Result<Progress> {
    let mut progress = repo.load()?;
    progress.unmark(path);
    repo.save(&progress)?;
    Ok(progress)
}
