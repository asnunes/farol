use super::model::{FileDiff, Scope};
use super::path::ReviewPath;
use crate::shared::error::Result;

/// Everything the app knows about git lives behind this.
///
/// It is the one port where the inversion earns its keep: a fake lets the map
/// and progress rules be tested against hand-built diffs, instead of building a
/// real repository for every case.
pub trait DiffSource: Send + Sync {
    /// The review window and the files inside it.
    fn scope(&self) -> Result<&Scope>;

    /// Full diff of one file across the review window.
    fn file_diff(&self, path: &str) -> Result<FileDiff>;

    /// Diff of one file between two arbitrary commits. Derivation uses this to
    /// learn how lines moved between the previous map's commit and now.
    /// `Ok(None)` means the file is identical between the two.
    fn file_diff_between(&self, from: &str, to: &str, path: &str) -> Result<Option<FileDiff>>;

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

    /// Commit the working tree is on right now.
    fn head_sha(&self) -> Result<String>;

    /// How many commits `HEAD` is ahead of `sha`. Powers "the map is 2 commits
    /// behind" without storing a pointer.
    fn commits_ahead_of(&self, sha: &str) -> Result<u32>;

    /// Whether `sha` is an ancestor of `HEAD` — used to walk back to the newest
    /// commit that still has a map.
    fn is_ancestor(&self, sha: &str) -> Result<bool>;
}
