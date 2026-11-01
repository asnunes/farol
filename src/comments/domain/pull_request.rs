//! The pull request the review is about: finding it, and opening one when
//! there is none yet.

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Where the pull request is, and opening one.
///
/// Its own port, apart from the pending review and the read marks: finding a
/// pull request changes when forks or branch lookups do, which has nothing to
/// do with how a comment is written or a file marked.
pub trait PullRequests: Send + Sync {
    /// What stands between the reviewer and the pull request, if anything.
    /// Looked up by the commit under review first, which is what finds the pull
    /// request of a fork or of a branch checked out under another name, and by
    /// the branch name after.
    fn readiness(&self, branch: &str, head: &str) -> Result<Readiness>;

    /// Open a pull request from `branch` into `base`. The branch has to be on
    /// the host already.
    fn open_pull(&self, base: &str, branch: &str, draft: &PullDraft) -> Result<OpenedPull>;
}

/// What the pull request will say, before it is opened: written by the
/// reviewer in the page, or by the agent when Generate is pressed.
///
/// Kept beside the review rather than only in the page, because the agent
/// writes it from its own process and the page has to pick it up from there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullDraft {
    pub title: String,
    pub body: String,
}

/// Where the draft lives between the agent writing it and the page reading it.
pub trait PullDraftStore: Send + Sync {
    fn read(&self) -> Result<Option<PullDraft>>;
    fn write(&self, draft: &PullDraft) -> Result<()>;
    /// Once the pull request is open, the draft has nothing left to say.
    fn clear(&self) -> Result<()>;
}

/// Sending the branch to the host, so a pull request can be opened on it.
///
/// A port of its own because it is git talking to the remote, not the host's
/// API: it runs with the reviewer's own git credentials, whatever they are.
pub trait BranchPusher: Send + Sync {
    fn push(&self, branch: &str) -> Result<()>;
}

/// The pull request the host opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedPull {
    pub number: u32,
    pub url: String,
}

/// How the commit under review stands against the pull request's, when they
/// differ. Each case has a different way out, and only one of them is a push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    /// The working tree has changes no commit holds yet.
    Uncommitted,
    /// The branch has commits the pull request does not: a push brings it up.
    Unpushed { commits: u32 },
    /// The pull request has commits this checkout does not, or the two went
    /// apart: a pull, and reading what came in.
    Newer,
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
        /// Its page, for the reader to open it from the review.
        url: String,
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

impl Readiness {
    /// Whether the review can go. The one question with a yes or no answer, and
    /// it is asked of the state rather than instead of it: every caller that
    /// hears no goes on to say which no it was.
    pub fn can_send(&self) -> bool {
        matches!(self, Readiness::Ready { .. })
    }
}
