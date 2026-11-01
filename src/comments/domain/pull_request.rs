//! Opening the pull request from the review, when there is none yet.

use serde::{Deserialize, Serialize};

use crate::error::Result;

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
