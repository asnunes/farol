use super::model::{FileDiff, Scope};
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

    /// Commit the working tree is on right now.
    fn head_sha(&self) -> Result<String>;

    /// How many commits `HEAD` is ahead of `sha`. Powers "the map is 2 commits
    /// behind" without storing a pointer.
    fn commits_ahead_of(&self, sha: &str) -> Result<u32>;

    /// Whether `sha` is an ancestor of `HEAD` — used to walk back to the newest
    /// commit that still has a map.
    fn is_ancestor(&self, sha: &str) -> Result<bool>;
}
