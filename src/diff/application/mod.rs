//! Three services over three ports.
//!
//! They were one, and it had three reasons to change: what is under review,
//! what the changes say, and where commits sit relative to each other. Keeping
//! them apart also stops a consumer from depending on more than it uses —
//! recording that a file was read needs a content hash, not the ability to walk
//! history.

use std::sync::Arc;

use crate::diff::domain::{
    CommitHistorySource, FileDiff, FileDiffSource, ReviewPath, ReviewScopeSource, Scope,
};
use crate::shared::error::Result;

/// What is under review, and turning raw paths into proven ones.
#[derive(Clone)]
pub struct ReviewScope {
    source: Arc<dyn ReviewScopeSource>,
}

impl ReviewScope {
    pub fn new(source: Arc<dyn ReviewScopeSource>) -> Self {
        Self { source }
    }

    pub fn get(&self) -> Result<&Scope> {
        self.source.scope()
    }

    pub fn path(&self, raw: &str) -> Result<ReviewPath> {
        self.source.review_path(raw)
    }

    /// Resolve several at once, failing on the first that is not under review.
    pub fn paths(&self, raw: &[String]) -> Result<Vec<ReviewPath>> {
        raw.iter().map(|p| self.path(p)).collect()
    }
}

/// The changes themselves.
#[derive(Clone)]
pub struct FileDiffs {
    source: Arc<dyn FileDiffSource>,
}

impl FileDiffs {
    pub fn new(source: Arc<dyn FileDiffSource>) -> Self {
        Self { source }
    }

    pub fn of(&self, path: &str) -> Result<FileDiff> {
        self.source.file_diff(path)
    }

    pub fn between(&self, from: &str, to: &str, path: &str) -> Result<Option<FileDiff>> {
        self.source.file_diff_between(from, to, path)
    }

    /// What viewed-state invalidation keys on: the file as it stands after the
    /// change, never the diff text.
    pub fn content_hash(&self, path: &str) -> Result<String> {
        Ok(self.of(path)?.new_content_hash)
    }
}

/// Where commits sit relative to one another.
#[derive(Clone)]
pub struct CommitHistory {
    source: Arc<dyn CommitHistorySource>,
}

impl CommitHistory {
    pub fn new(source: Arc<dyn CommitHistorySource>) -> Self {
        Self { source }
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
