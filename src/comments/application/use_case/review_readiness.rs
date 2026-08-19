use std::sync::Arc;

use crate::comments::domain::{Readiness, ReviewPublisher};
use crate::diff::application::ReviewScope;
use crate::error::Result;

/// Ask whether the review could be sent, and if not, what is in the way.
///
/// A question rather than a command, and the only one the screen asks on its
/// own: the answer changes outside farol — a token pasted into a file, a branch
/// pushed from a terminal, a pull request opened in a browser — so the reader
/// has to be able to ask again without doing anything irreversible.
#[derive(Clone)]
pub struct ReviewReadiness {
    publisher: Arc<dyn ReviewPublisher>,
    scope: ReviewScope,
}

impl ReviewReadiness {
    pub fn new(publisher: Arc<dyn ReviewPublisher>, scope: ReviewScope) -> Self {
        Self { publisher, scope }
    }

    pub fn execute(&self) -> Result<Standing> {
        let branch = self.scope.get()?.branch.clone();
        Ok(Standing {
            readiness: self.publisher.readiness(&branch)?,
            branch,
        })
    }
}

/// Where the review stands with the host, and the branch the answer is about.
///
/// The branch travels with it because every way of not being ready is explained
/// with the branch name in it — the command to push it, the page that opens a
/// pull request for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub branch: String,
    pub readiness: Readiness,
}
