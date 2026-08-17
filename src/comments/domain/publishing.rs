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
    Ready { pull_request: u32, head: String },

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

impl Verdict {
    /// Whether the summary is required. Approving needs no words — the other
    /// two are somebody being asked to do something, and "why" is the whole of
    /// it.
    pub fn needs_summary(&self) -> bool {
        !matches!(self, Verdict::Approve)
    }
}

/// The GitHub credential, kept where the review cannot see it.
///
/// A port for one reason: publishing is tested against a fake, and a fake that
/// reached for the real file would read the person's actual token.
pub trait Credentials: Send + Sync {
    fn token(&self) -> Result<Option<String>>;

    fn set(&self, token: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_approving_can_go_out_without_a_word() {
        assert!(!Verdict::Approve.needs_summary());
        assert!(Verdict::Comment.needs_summary());
        assert!(Verdict::RequestChanges.needs_summary());
    }
}
