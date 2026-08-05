//! Three ports rather than one, because asking git something has three
//! different shapes and each caller wants only one of them.
//!
//! One implementation answers all three — `GixSource` shares a blob cache
//! across them — but a consumer depends on the slice it uses. Deriving a map
//! needs history; showing a file does not, and its signature says so.

use super::model::{FileDiff, Scope};
use super::path::ReviewPath;
use crate::shared::error::Result;

/// What is under review, and whether a path is part of it.
pub trait ReviewScopeSource: Send + Sync {
    fn scope(&self) -> Result<&Scope>;

    /// Line count of the file as it stands after the change. Range validation
    /// needs it to reject a note pointing past the end of the file.
    fn file_line_count(&self, path: &str) -> Result<u32>;

    /// Turn a raw path into one proven to be under review. This is the only
    /// constructor of [`ReviewPath`], so anything downstream that takes one is
    /// guaranteed the check happened.
    fn review_path(&self, raw: &str) -> Result<ReviewPath> {
        let scope = self.scope()?;
        if !scope.contains(raw) {
            return Err(scope.reject(raw));
        }
        Ok(ReviewPath::proven(raw, self.file_line_count(raw)?))
    }
}

/// The changes themselves.
pub trait FileDiffSource: Send + Sync {
    /// Full diff of one file across the review window.
    fn file_diff(&self, path: &str) -> Result<FileDiff>;

    /// Git's own name for the file as it stands after the change: the blob id.
    ///
    /// Separate from `file_diff` because recording that a file was read must
    /// not pay for diffing it — and because the id is read off the tree entry,
    /// while the diff has to be computed.
    fn content_hash(&self, path: &str) -> Result<String>;

    /// Diff of one file between two arbitrary commits. Derivation uses this to
    /// learn how lines moved between the previous map's commit and now.
    /// `Ok(None)` means the file is identical between the two.
    fn file_diff_between(&self, from: &str, to: &str, path: &str) -> Result<Option<FileDiff>>;
}

/// Where commits sit relative to one another — what versioning a map needs and
/// nothing else does.
pub trait CommitHistorySource: Send + Sync {
    /// Commit the working tree is on right now.
    fn head_sha(&self) -> Result<String>;

    /// How many commits `HEAD` is ahead of `sha`. Powers "the map is 2 commits
    /// behind" without storing a pointer.
    fn commits_ahead_of(&self, sha: &str) -> Result<u32>;

    /// Whether `sha` is an ancestor of `HEAD` — used to walk back to the newest
    /// commit that still has a map.
    fn is_ancestor(&self, sha: &str) -> Result<bool>;
}
