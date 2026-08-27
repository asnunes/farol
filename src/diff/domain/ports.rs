//! Three ports rather than one, because asking git something has three
//! different shapes and each caller wants only one of them.
//!
//! One implementation answers all three — `GixSource` shares a blob cache
//! across them — but a consumer depends on the slice it uses. Deriving a map
//! needs history; showing a file does not, and its signature says so.

use super::model::{FileDiff, Scope};
use super::path::ReviewPath;
use crate::error::Result;

/// What is under review, and whether a path is part of it.
pub trait ReviewScopeSource: Send + Sync {
    fn scope(&self) -> Result<&Scope>;

    /// Line count of the file as it stands after the change. Range validation
    /// needs it to reject a note pointing past the end of the file.
    fn file_line_count(&self, path: &str) -> Result<u32>;

    /// The file as it stands after the change, from `from` to `to` inclusive,
    /// one string per line.
    ///
    /// Beside the count rather than with the diff because it answers about the
    /// file and not about the change: the screen asks for it to show what the
    /// diff never printed. A range past the end is trimmed to what is there,
    /// since the caller is a reader scrolling, not a caller making a claim.
    fn file_lines(&self, path: &str, from: u32, to: u32) -> Result<Vec<String>>;

    /// Turn a raw path into one proven to be under review. This is the only
    /// constructor of [`ReviewPath`], so anything downstream that takes one is
    /// guaranteed the check happened.
    fn review_path(&self, raw: &str) -> Result<ReviewPath> {
        let scope = self.scope()?;
        if !scope.contains(raw) {
            return Err(scope.reject(raw).into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeDiffSource;

    #[test]
    fn a_path_under_review_comes_back_proven_and_carrying_its_length() {
        // The line count rides along because range validation needs it and
        // this is the moment it is known.
        let source = FakeDiffSource::with_paths(&["a.rs"]).with_line_count("a.rs", 42);

        let path = source.review_path("a.rs").unwrap();

        assert_eq!(path.as_str(), "a.rs");
        assert_eq!(path.lines(), 42);
    }

    #[test]
    fn a_path_outside_the_review_never_becomes_one() {
        // This is the only constructor of `ReviewPath`, so refusing here is
        // what makes every use case downstream unable to receive a bad path.
        let source = FakeDiffSource::with_paths(&["a.rs"]);

        let err = source.review_path("elsewhere.rs").unwrap_err();

        assert!(err.to_string().contains("elsewhere.rs"), "{err}");
    }

    #[test]
    fn the_rejection_carries_the_near_misses_back() {
        // The reader is the session writing the map, and a hallucinated path
        // is the mistake it makes most.
        let source = FakeDiffSource::with_paths(&["src/store/db.rs"]);

        let err = source.review_path("src/stores/db.rs").unwrap_err();

        assert!(err.to_string().contains("src/store/db.rs"), "{err}");
    }
}
