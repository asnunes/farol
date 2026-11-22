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

    /// Every file of the pull request and whether the reviewer has marked it
    /// as read, however many pages the host answers in.
    ///
    /// Every file, not only the marked ones: the review here and the pull
    /// request can disagree about which files there are (a deletion here can
    /// be a rename there), and a file the pull request does not have is one no
    /// mark can be put on.
    fn files(&self, pull: &str) -> Result<Vec<PullFile>>;
}

/// One file of the pull request, as the host lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullFile {
    pub path: String,
    pub viewed: bool,
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
