//! The pending review, and sending it where the rest of the team reads it.

use crate::comments::domain::Lines;
use crate::error::Result;

/// The reviewer's pending review on the pull request, and submitting it.
///
/// A trait so the use cases never learn what a pull request is over the wire,
/// and so a test can assert what would have been sent without the network.
///
/// The pending comments are on the host from the moment they are written, in
/// the reviewer's own pending review — the same place the pull request's page
/// keeps them. farol holds their ids and nothing else, so there is no local
/// copy to reconcile and nothing half-sent to recover when a call fails.
pub trait PendingReview: Send + Sync {
    /// Put a comment in the reviewer's pending review on `pull`, starting one
    /// at `head` when there is none. Answers with the comment's id.
    fn add_pending(&self, pull: &str, head: &str, at: &Lines, body: &str) -> Result<String>;

    fn edit_pending(&self, id: &str, body: &str) -> Result<()>;

    /// Taking a pending comment back. One already gone is not an error: the
    /// reviewer may have deleted it on the pull request's page.
    fn remove_pending(&self, id: &str) -> Result<()>;

    /// What the host holds for these comment ids, in the same order.
    fn remote(&self, ids: &[String]) -> Result<Vec<RemoteComment>>;

    /// Every comment in the reviewer's pending review on `pull`, wherever it
    /// was written: here, on the pull request's page, or by a thread that has
    /// since been closed. The pending review is the list; farol keeps none.
    ///
    /// Every one of them, including those on a file as a whole or on lines a
    /// later push made outdated: a submit sends them all, so the reviewer is
    /// shown them all. GitHub's answer stops at its first 100 threads.
    fn pending_comments(&self, pull: &str) -> Result<Vec<PendingComment>>;

    /// Submit the pending review with the verdict and summary, or post the
    /// verdict alone when nothing is pending. Answers with where it landed.
    fn submit(&self, review: &Review) -> Result<String>;
}

/// A comment in the pending review, with where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingComment {
    pub id: String,
    pub body: String,
    pub at: PendingAt,
}

/// Where a pending comment sits. Not always on lines: GitHub also holds
/// comments on a file as a whole, and drops the line of one a later push left
/// outdated. Both still go out with the review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAt {
    Lines(Lines),
    File { path: String, outdated: bool },
}

impl PendingAt {
    pub fn path(&self) -> &str {
        match self {
            Self::Lines(lines) => &lines.path,
            Self::File { path, .. } => path,
        }
    }

    pub fn lines(&self) -> Option<&Lines> {
        match self {
            Self::Lines(lines) => Some(lines),
            Self::File { .. } => None,
        }
    }

    /// Where it is, the way a person reads it.
    pub fn at(&self) -> String {
        match self {
            Self::Lines(lines) => lines.at(),
            Self::File {
                path,
                outdated: true,
            } => format!("{path} (outdated)"),
            Self::File { path, .. } => format!("{path} (file)"),
        }
    }
}

/// One comment as the host has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteComment {
    pub id: String,
    pub state: RemoteState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteState {
    /// In the pending review: only the reviewer sees it.
    Pending { body: String },
    /// Submitted with a review.
    Published { body: String, url: String },
    /// Deleted on the pull request's page, or never there.
    Gone,
}

/// A review as it is submitted: the verdict and the summary the reviewer wrote
/// at the end. The comments are already on the host, in the pending review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    /// The host's id for the pull request.
    pub pull: String,
    /// The commit a verdict with nothing pending is posted against.
    pub head: String,
    pub summary: String,
    pub verdict: Verdict,
}

/// What the review says about the change as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Comment,
    RequestChanges,
    Approve,
}

impl Verdict {
    /// Whether the summary is required.
    ///
    /// Only of the one that asks for work. Commenting and approving are
    /// carried by whatever goes with them, and a pending review is submitted
    /// without the host asking for a summary either.
    /// Asking for changes is a demand, and a demand with no words leaves the
    /// author reading line notes to work out what would satisfy it.
    pub fn needs_summary(&self) -> bool {
        matches!(self, Verdict::RequestChanges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_asking_for_changes_needs_a_word_of_its_own() {
        assert!(Verdict::RequestChanges.needs_summary());
        assert!(!Verdict::Approve.needs_summary());
        // The comments are the review. Submitted from a pending review, GitHub
        // does not ask for a summary here either.
        assert!(!Verdict::Comment.needs_summary());
    }
}
