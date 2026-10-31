use std::sync::Arc;

use crate::comments::application::{Comments, PullRequestGate, ReviewModes};
use crate::comments::domain::{
    ModeChoice, PendingComment, Readiness, RemoteState, ReviewPublisher,
};
use crate::error::Result;

/// Where the review stands with the pull request: whether there is one to
/// write on, whose it is and so which mode this is, and what each thread's
/// pending comment looks like there now.
///
/// One question for the screen and for `farol github status`, because both
/// draw the same thing and two assemblies would drift.
#[derive(Clone)]
pub struct ReviewReadiness {
    pull: PullRequestGate,
    modes: ReviewModes,
    comments: Comments,
    publisher: Arc<dyn ReviewPublisher>,
    host: Option<String>,
    read: crate::lenses::application::ListReadFiles,
    drafts: std::sync::Arc<dyn crate::comments::domain::PullDraftStore>,
}

impl ReviewReadiness {
    pub fn new(
        pull: PullRequestGate,
        modes: ReviewModes,
        comments: Comments,
        publisher: Arc<dyn ReviewPublisher>,
        host: Option<String>,
        read: crate::lenses::application::ListReadFiles,
        drafts: std::sync::Arc<dyn crate::comments::domain::PullDraftStore>,
    ) -> Self {
        Self {
            pull,
            modes,
            comments,
            publisher,
            host,
            read,
            drafts,
        }
    }

    pub fn execute(&self) -> Result<Standing> {
        let branch = self.pull.branch()?;
        let readiness = self.pull.readiness()?;
        let mode = self.modes.detect(&readiness)?;
        let (pending, loose, pending_failed) = match &readiness {
            Readiness::Ready { id, .. } => match self.pending(id) {
                Ok((pending, loose)) => (pending, loose, None),
                // The rest of the answer is still worth having: the screen
                // draws the threads and says the pending ones could not be read.
                Err(error) => (vec![], vec![], Some(error.to_string())),
            },
            _ => (vec![], vec![], None),
        };
        Ok(Standing {
            host: self.host.clone(),
            branch,
            mode,
            pending,
            loose,
            pending_failed,
            read: self.read.execute()?,
            // Only while there is a pull request to open: once it exists, the
            // draft has said what it had to.
            draft: match readiness {
                Readiness::NoPullRequest { .. } | Readiness::BranchNotPushed => {
                    self.drafts.read()?
                }
                _ => None,
            },
            readiness,
        })
    }

    /// Each thread's comment as the host holds it, and the rest of the pending
    /// review: what was written on the pull request's page, and what outlived
    /// the thread it came from.
    fn pending(&self, pull: &str) -> Result<(Vec<ThreadPending>, Vec<PendingComment>)> {
        let threads: Vec<_> = self
            .comments
            .every()?
            .into_iter()
            .filter_map(|thread| thread.pending.clone().map(|pending| (thread, pending)))
            .collect();
        let ids: Vec<String> = threads
            .iter()
            .map(|(_, pending)| pending.id.clone())
            .collect();
        let remote = self.publisher.remote(&ids)?;
        let loose = self
            .publisher
            .pending_comments(pull)?
            .into_iter()
            .filter(|comment| !ids.contains(&comment.id))
            .collect();
        let pending = threads
            .into_iter()
            .zip(remote)
            .map(|((thread, _), remote)| ThreadPending {
                thread: thread.id.clone(),
                at: thread.at(),
                state: remote.state,
            })
            .collect();
        Ok((pending, loose))
    }
}

/// Where the review stands with the host, and the branch the answer is about.
///
/// The branch travels with it because every way of not being ready is explained
/// with the branch name in it — the command to push it, the page that opens a
/// pull request for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub host: Option<String>,
    pub branch: String,
    pub readiness: Readiness,
    pub mode: ModeChoice,
    pub pending: Vec<ThreadPending>,
    /// Pending comments no thread here points at.
    pub loose: Vec<PendingComment>,
    /// Why the pending comments could not be read, when they could not.
    pub pending_failed: Option<String>,
    pub read: Vec<String>,
    /// What the pull request will say, while there is none to open yet.
    pub draft: Option<crate::comments::domain::PullDraft>,
}

/// One thread's pending comment, as the pull request has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadPending {
    pub thread: String,
    /// Where the thread sits, the way a person would type it.
    pub at: String,
    pub state: RemoteState,
}
