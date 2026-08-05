use std::sync::Arc;

use crate::diff::domain::{DiffSource, FileDiff, ReviewPath, Scope};
use crate::shared::error::Result;

/// Everything the rest of the program needs to ask about the code under review.
///
/// It owns the port rather than borrowing it, which is what lets the services
/// above it be stored in a struct instead of rebuilt on every call. It also
/// means no caller outside `application` and `infra` ever names `DiffSource`:
/// the entry points see a service, not a port.
#[derive(Clone)]
pub struct DiffService {
    source: Arc<dyn DiffSource>,
}

impl DiffService {
    pub fn new(source: Arc<dyn DiffSource>) -> Self {
        Self { source }
    }

    pub fn scope(&self) -> Result<&Scope> {
        self.source.scope()
    }

    /// Turn a raw path into one proven to be under review.
    pub fn review_path(&self, raw: &str) -> Result<ReviewPath> {
        self.source.review_path(raw)
    }

    /// Resolve several at once, failing on the first that is not under review.
    pub fn review_paths(&self, raw: &[String]) -> Result<Vec<ReviewPath>> {
        raw.iter().map(|p| self.review_path(p)).collect()
    }

    pub fn file_diff(&self, path: &str) -> Result<FileDiff> {
        self.source.file_diff(path)
    }

    pub fn file_diff_between(&self, from: &str, to: &str, path: &str) -> Result<Option<FileDiff>> {
        self.source.file_diff_between(from, to, path)
    }

    /// How far `HEAD` has moved past `sha`. Distance is informational, so a
    /// history that cannot be walked reads as "not behind" rather than failing
    /// the command that asked.
    pub fn commits_ahead_of(&self, sha: &str) -> u32 {
        self.source.commits_ahead_of(sha).unwrap_or(0)
    }

    pub fn is_ancestor(&self, sha: &str) -> Result<bool> {
        self.source.is_ancestor(sha)
    }
}
