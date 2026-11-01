use crate::diff::application::ReviewScope;
use crate::error::{Error, Result};
use crate::lenses::application::GetReviewWithLenses;
use crate::map::domain::MapError;
use crate::progress::application::ProgressStore;

/// List the files the reviewer has read, as GitHub is told about them. The
/// same answer the screen draws — ticked, or covered by the block's lenses —
/// so a file shown read is a file marked read on the pull request.
#[derive(Clone)]
pub struct ListReadFiles {
    progress: ProgressStore,
    scope: ReviewScope,
    review: GetReviewWithLenses,
}

impl ListReadFiles {
    pub fn new(progress: ProgressStore, scope: ReviewScope, review: GetReviewWithLenses) -> Self {
        Self {
            progress,
            scope,
            review,
        }
    }

    pub fn execute(&self) -> Result<Vec<String>> {
        match self.review.execute() {
            Ok((snapshot, lenses)) => Ok(lenses.read_paths(&snapshot)),
            // No map means no blocks, so nothing is covered; the ticks still are.
            Err(Error::Map(MapError::NoMap { .. })) => {
                self.progress.current_paths(&self.scope.get()?.files)
            }
            Err(e) => Err(e),
        }
    }
}
