//! Sending the review somewhere the rest of the team will read it.
//!
//! Two questions, because they are asked at different moments and one of them
//! is asked constantly: *can this be sent* is answered while the reviewer is
//! still reading, and *send this* once, at the end.

use crate::comments::domain::Comment;
use crate::error::Result;

/// Where a finished review goes. A trait so the use case never learns what a
/// pull request is over the wire — and so a test can assert the review that
/// would have been sent without touching the network.
pub trait ReviewPublisher: Send + Sync {
    /// What stands between the reviewer and sending, if anything.
    fn readiness(&self, branch: &str) -> Result<Readiness>;

    /// Send it. Answers with where the review can be read.
    fn publish(&self, review: &Review) -> Result<String>;

    /// Mark these files as read on the pull request, for whoever the token
    /// belongs to. Answers with how many the host took.
    ///
    /// Its own verb rather than part of `publish`, because the two have
    /// different consequences: the review is posted once and cannot be taken
    /// back, while marks are a convenience that can fail without costing
    /// anything. Folded together, the second failing would hide inside the
    /// first succeeding.
    fn mark_read(&self, pull: &str, paths: &[String]) -> Result<usize>;
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
    },

    /// The repository has no remote, so there is no host to publish to. Not a
    /// step on the way to being ready — a different situation entirely.
    NoRemote,

    /// farol has no token to speak with.
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

/// A review as it goes out: the summary the reviewer wrote at the end, the
/// verdict, and the comments that have not gone yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    pub pull_request: u32,
    /// The commit the comments were written against, sent along so the host
    /// anchors them where they were read.
    pub head: String,
    pub summary: String,
    pub verdict: Verdict,
    pub comments: Vec<Comment>,
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
    /// carried by whatever goes with them, and the comments are sent as a
    /// draft first precisely so that the host asks for no summary either.
    /// Asking for changes is a demand, and a demand with no words leaves the
    /// author reading line notes to work out what would satisfy it.
    pub fn needs_summary(&self) -> bool {
        matches!(self, Verdict::RequestChanges)
    }
}

/// A credential is available only for the host its owner authorized.
///
/// A port for one reason: publishing is tested against a fake, and a fake that
/// reached for the real file would read the person's actual token.
pub trait Credentials: Send + Sync {
    fn token(&self, host: &str) -> Result<Option<String>>;

    fn set(&self, host: &str, token: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_asking_for_changes_needs_a_word_of_its_own() {
        assert!(Verdict::RequestChanges.needs_summary());
        assert!(!Verdict::Approve.needs_summary());
        // The comments are the review. Sent as a draft and submitted after,
        // GitHub does not ask for a summary here either.
        assert!(!Verdict::Comment.needs_summary());
    }
}
