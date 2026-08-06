use crate::diff::application::FileDiffs;
use crate::diff::domain::FileDiff;
use crate::shared::error::Result;

/// One file's diff, for the pane on the right.
#[derive(Clone)]
pub struct GetFileDiff {
    diffs: FileDiffs,
}

impl GetFileDiff {
    pub fn new(diffs: FileDiffs) -> Self {
        Self { diffs }
    }

    pub fn execute(&self, path: &str) -> Result<FileDiff> {
        self.diffs.of(path)
    }
}
