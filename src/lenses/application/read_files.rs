use crate::diff::application::ReviewScope;
use crate::error::{Error, Result};
use crate::lenses::application::GetReviewWithLenses;
use crate::map::domain::MapError;
use crate::progress::application::ProgressStore;

/// A **service**: the files the reviewer has read, as GitHub is told about
/// them. The same answer the screen draws — ticked, or covered by the block's
/// lenses — so a file shown read is a file marked read on the pull request.
#[derive(Clone)]
pub struct ReadFiles {
    progress: ProgressStore,
    scope: ReviewScope,
    review: GetReviewWithLenses,
}

impl ReadFiles {
    pub fn new(progress: ProgressStore, scope: ReviewScope, review: GetReviewWithLenses) -> Self {
        Self {
            progress,
            scope,
            review,
        }
    }

    pub fn paths(&self) -> Result<Vec<String>> {
        let mut read = self.progress.current_paths(&self.scope.get()?.files)?;
        // No map means no blocks, so nothing is covered; the ticks still are.
        match self.review.execute() {
            Ok((snapshot, lenses)) => {
                for path in lenses.read_paths(&snapshot) {
                    if !read.contains(&path) {
                        read.push(path);
                    }
                }
            }
            Err(Error::Map(MapError::NoMap { .. })) => {}
            Err(e) => return Err(e),
        }
        Ok(read)
    }
}
