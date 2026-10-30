//! The pull request the review is about, or the reason there is none to use.

use std::sync::Arc;

use crate::comments::domain::{CommentError, Readiness, ReviewPublisher};
use crate::diff::application::{CommitHistory, ReviewScope};
use crate::error::Result;
use crate::shared::short;

/// Turns the host's answer into a pull request to act on, or into the refusal
/// that says what is in the way.
///
/// Every use case that touches the pull request asks this first. Each of them
/// used to spell out the same six arms, which is how two of them came to word
/// the same refusal differently.
#[derive(Clone)]
pub struct PullRequestGate {
    publisher: Arc<dyn ReviewPublisher>,
    scope: ReviewScope,
    history: CommitHistory,
}

impl PullRequestGate {
    pub fn new(
        publisher: Arc<dyn ReviewPublisher>,
        scope: ReviewScope,
        history: CommitHistory,
    ) -> Self {
        Self {
            publisher,
            scope,
            history,
        }
    }

    /// The branch under review, which every answer about the pull request is
    /// about.
    pub fn branch(&self) -> Result<String> {
        Ok(self.scope.get()?.branch.clone())
    }

    /// What the host says, as it says it.
    pub fn readiness(&self) -> Result<Readiness> {
        let scope = self.scope.get()?;
        self.publisher.readiness(&scope.branch, &scope.head_sha)
    }

    /// The open pull request, wherever its head is.
    pub fn open(&self) -> Result<OpenPull> {
        let branch = self.scope.get()?.branch.clone();
        match self.readiness()? {
            Readiness::Ready {
                pull_request,
                id,
                head,
                mine,
                ..
            } => Ok(OpenPull {
                number: pull_request,
                id,
                head,
                mine,
            }),
            Readiness::Ambiguous { numbers } => {
                Err(CommentError::AmbiguousPullRequest { numbers }.into())
            }
            Readiness::NoRemote => Err(CommentError::NoRemote.into()),
            Readiness::NoToken => Err(CommentError::NoToken.into()),
            Readiness::TokenRefused => Err(CommentError::TokenRefused.into()),
            Readiness::BranchNotPushed => Err(CommentError::BranchNotPushed { branch }.into()),
            Readiness::NoPullRequest { .. } => Err(CommentError::NoPullRequest { branch }.into()),
        }
    }

    /// The open pull request, showing the commit the reviewer is reading.
    ///
    /// A comment is anchored by line number, and against another commit it
    /// lands on code nobody read — quietly, under the reviewer's name. So
    /// anything written onto the pull request asks this, and the refusal says
    /// which way to move.
    pub fn at_head(&self) -> Result<OpenPull> {
        let scope = self.scope.get()?;
        if scope.dirty {
            return Err(CommentError::Uncommitted.into());
        }
        let pull = self.open()?;
        if pull.head != scope.head_sha {
            return Err(CommentError::HeadMoved {
                theirs: short(&pull.head),
                ours: short(&scope.head_sha),
                behind: !self.history.is_ancestor(&pull.head)?,
            }
            .into());
        }
        Ok(pull)
    }
}

/// An open pull request, as the use cases need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPull {
    pub number: u32,
    /// The host's id, which GraphQL addresses it by.
    pub id: String,
    pub head: String,
    /// Whether it is the reviewer's own.
    pub mine: bool,
}
