//! Sending the review somewhere the rest of the team will read it.

use crate::comments::domain::Comment;
use crate::error::Result;

/// Where a review goes, and where its pending comments live until it does.
///
/// A trait so the use cases never learn what a pull request is over the wire,
/// and so a test can assert what would have been sent without the network.
///
/// The pending comments are on the host from the moment they are written, in
/// the reviewer's own pending review — the same place the pull request's page
/// keeps them. farol holds their ids and nothing else, so there is no local
/// copy to reconcile and nothing half-sent to recover when a call fails.
pub trait ReviewPublisher: Send + Sync {
    /// What stands between the reviewer and the pull request, if anything.
    /// Looked up by the commit under review first, which is what finds the pull
    /// request of a fork or of a branch checked out under another name, and by
    /// the branch name after.
    fn readiness(&self, branch: &str, head: &str) -> Result<Readiness>;

    /// Put a comment in the reviewer's pending review on `pull`, starting one
    /// at `head` when there is none. Answers with the comment's id.
    fn add_pending(&self, pull: &str, head: &str, at: &Comment, body: &str) -> Result<String>;

    fn edit_pending(&self, id: &str, body: &str) -> Result<()>;

    /// Taking a pending comment back. One already gone is not an error: the
    /// reviewer may have deleted it on the pull request's page.
    fn remove_pending(&self, id: &str) -> Result<()>;

    /// What the host holds for these comment ids, in the same order.
    fn remote(&self, ids: &[String]) -> Result<Vec<RemoteComment>>;

    /// Submit the pending review with the verdict and summary, or post the
    /// verdict alone when nothing is pending. Answers with where it landed.
    fn submit(&self, review: &Review) -> Result<String>;

    /// Mark these files as read on the pull request, for whoever the token
    /// belongs to. Answers with how many the host took.
    ///
    /// Its own verb rather than part of `submit`, because the two have
    /// different consequences: the review is posted once and cannot be taken
    /// back, while marks are a convenience that can fail without costing
    /// anything. Folded together, the second failing would hide inside the
    /// first succeeding.
    fn mark_read(&self, pull: &str, paths: &[String]) -> Result<usize>;
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

/// Why the review cannot be sent yet — or that it can.
///
/// A reason rather than a flag, because each way of not being ready needs a
/// different thing done about it, and only the state says which. Told "not
/// ready", the reviewer would go looking for the wrong thing — and the wrong
/// thing here is a token they already have, or a pull request they cannot open
/// because the branch is not there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    /// The pull request is there, and this is the commit it is showing.
    ///
    /// `id` is the host's own name for it, which marking a file as read needs
    /// and the number cannot stand in for. It comes back in the same answer as
    /// the other two and goes no further than the use case: the screen is told
    /// the number, which is what a person recognises.
    Ready {
        pull_request: u32,
        id: String,
        head: String,
        /// Whether the pull request is the reviewer's own.
        ///
        /// GitHub does not let anybody approve or ask changes on their own, and
        /// answers a bare "Unprocessable Entity" when they try. Knowing first
        /// is what lets farol offer the verdict that exists instead of the two
        /// that do not.
        mine: bool,
        /// Who opened it, which is what decides the review mode.
        author: String,
    },

    /// More than one open pull request shows this commit, and farol will not
    /// pick one at random.
    Ambiguous { numbers: Vec<u32> },

    /// The repository has no remote, so there is no host to publish to. Not a
    /// step on the way to being ready — a different situation entirely.
    NoRemote,

    /// GitHub CLI has no credential for this host.
    NoToken,

    /// It has one and the host would not take it.
    TokenRefused,

    /// The host has never heard of this branch. Nothing to open a pull request
    /// against, so this state is upstream of the next one rather than beside it.
    BranchNotPushed,

    /// The branch is there and no pull request is open on it. `open_at` is the
    /// page that starts one, prefilled with the branch.
    NoPullRequest { open_at: String },
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

impl Readiness {
    /// Whether the review can go. The one question with a yes or no answer, and
    /// it is asked of the state rather than instead of it: every caller that
    /// hears no goes on to say which no it was.
    pub fn can_send(&self) -> bool {
        matches!(self, Readiness::Ready { .. })
    }
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

/// Credentials are read only for the selected host; authentication is managed externally.
///
/// A port for one reason: publishing is tested against a fake, and a fake that
/// consulted the real login would read the person's actual token.
pub trait Credentials: Send + Sync {
    fn token(&self, host: &str) -> Result<Option<String>>;
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
