//! Three services over three ports.
//!
//! They were one, and it had three reasons to change: what is under review,
//! what the changes say, and where commits sit relative to each other. Keeping
//! them apart also stops a consumer from depending on more than it uses —
//! recording that a file was read needs a content hash, not the ability to walk
//! history.

mod check_head;
pub use check_head::CheckHead;

use std::sync::Arc;

use crate::diff::domain::{
    CommitHistorySource, FileDiff, FileDiffSource, ReviewPath, ReviewScopeSource, Scope,
};
use crate::error::Result;

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

    /// A stretch of the file as it now reads, proven to be under review first.
    pub fn lines(&self, raw: &str, from: u32, to: u32) -> Result<Vec<String>> {
        let path = self.path(raw)?;
        self.source.file_lines(path.as_str(), from, to)
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
    /// change, never the diff text. It is git's blob id, so identical content
    /// is identical everywhere — including across a rebase that only moved it.
    pub fn content_hash(&self, path: &str) -> Result<String> {
        self.source.content_hash(path)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::testing::FakeDiffSource;

    fn scope_over(paths: &[&str]) -> ReviewScope {
        ReviewScope::new(Arc::new(FakeDiffSource::with_paths(paths)))
    }

    #[test]
    fn a_path_under_review_comes_back_carrying_how_long_the_file_is() {
        let scope = ReviewScope::new(Arc::new(
            FakeDiffSource::with_paths(&["a.rs"]).with_line_count("a.rs", 42),
        ));

        let path = scope.path("a.rs").unwrap();

        assert_eq!(path.as_str(), "a.rs");
        assert_eq!(
            path.lines(),
            42,
            "the line count rides along so a range check never has to ask again"
        );
    }

    #[test]
    fn a_path_outside_the_review_cannot_be_turned_into_one() {
        // This is the only constructor of ReviewPath, so refusing here is what
        // makes every use case downstream unable to receive a bad path at all.
        let err = scope_over(&["a.rs"]).path("elsewhere.rs").unwrap_err();

        assert!(err.to_string().contains("elsewhere.rs"), "{err}");
    }

    #[test]
    fn resolving_several_paths_fails_on_the_first_bad_one() {
        // Half a block is worse than none: the caller is told before anything
        // is written, and told about the path it actually got wrong.
        let err = scope_over(&["a.rs", "b.rs"])
            .paths(&["a.rs".into(), "nope.rs".into(), "b.rs".into()])
            .unwrap_err();

        assert!(err.to_string().contains("nope.rs"), "{err}");
    }

    #[test]
    fn resolving_several_good_paths_keeps_the_order_they_were_given() {
        let paths = scope_over(&["a.rs", "b.rs"])
            .paths(&["b.rs".into(), "a.rs".into()])
            .unwrap();

        let names: Vec<&str> = paths.iter().map(|p| p.as_str()).collect();
        assert_eq!(names, vec!["b.rs", "a.rs"]);
    }

    #[test]
    fn the_content_hash_is_of_the_file_after_the_change() {
        let diffs = FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&["a.rs"])));

        assert_eq!(diffs.content_hash("a.rs").unwrap(), "hash-of-a.rs");
    }

    #[test]
    fn asking_for_a_hash_of_a_file_outside_the_review_fails() {
        let diffs = FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&["a.rs"])));

        assert!(diffs.content_hash("elsewhere.rs").is_err());
    }

    /// A history that cannot be walked at all — a shallow clone, a grafted
    /// commit. Only `commits_ahead_of` is exercised through it.
    struct BrokenHistory;

    impl CommitHistorySource for BrokenHistory {
        fn head_sha(&self) -> Result<String> {
            // Required by the port; these tests only ask about distance and
            // ancestry, so nothing calls it.
            unreachable!("BrokenHistory is only asked about distance and ancestry")
        }

        fn commits_ahead_of(&self, _sha: &str) -> Result<u32> {
            Err(Error::Message("shallow clone".into()))
        }
        fn is_ancestor(&self, _sha: &str) -> Result<bool> {
            Err(Error::Message("shallow clone".into()))
        }
    }

    #[test]
    fn a_history_that_cannot_be_walked_reads_as_not_behind() {
        // Distance is decoration on `map show`. Failing the command over it
        // would take the map away from someone whose repo is merely shallow.
        let history = CommitHistory::new(Arc::new(BrokenHistory));

        assert_eq!(history.commits_ahead_of("abc123"), 0);
    }

    #[test]
    fn whether_a_commit_is_an_ancestor_is_not_swallowed() {
        // Unlike distance, this one decides which map is current — guessing
        // would hand the reviewer the wrong version.
        let history = CommitHistory::new(Arc::new(BrokenHistory));

        assert!(history.is_ancestor("abc123").is_err());
    }
}
