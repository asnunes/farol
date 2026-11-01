//! Which files the reviewer has marked as read on the pull request.

use crate::error::Result;

/// The read marks on the pull request, for whoever the token belongs to.
///
/// Apart from the pending review because the two have different
/// consequences: a review is posted once and cannot be taken back, while marks
/// are a convenience that can fail without costing anything. Folded together,
/// the second failing would hide inside the first succeeding.
pub trait ReadMarks: Send + Sync {
    /// Mark these files as read. Answers with how many the host took.
    fn mark_read(&self, pull: &str, paths: &[String]) -> Result<usize>;

    /// Take the read mark off these files. Answers with how many the host
    /// took.
    fn mark_unread(&self, pull: &str, paths: &[String]) -> Result<usize>;

    /// The files the reviewer has marked as read. GitHub's answer stops at
    /// the pull request's first 100 files.
    fn viewed(&self, pull: &str) -> Result<Vec<String>>;
}

/// The files the reviewer has read here, in the review: what the read marks on
/// the pull request are made to say.
///
/// A port, rather than the lenses' use case held directly, so the comments'
/// use cases need not build the map, the lenses and the progress to learn a
/// list of paths, and their tests can hand one in.
pub trait ReadFiles: Send + Sync {
    fn read(&self) -> Result<Vec<String>>;
}
