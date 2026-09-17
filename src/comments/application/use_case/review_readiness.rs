use std::sync::Arc;

use crate::comments::domain::{Readiness, ReviewPublisher};
use crate::diff::application::ReviewScope;
use crate::error::Result;

/// Ask whether the review could be sent, and if not, what is in the way.
///
/// Read-only preparation for the publication skill, including the marks it will send.
#[derive(Clone)]
pub struct ReviewReadiness {
    publisher: Arc<dyn ReviewPublisher>,
    scope: ReviewScope,
    host: Option<String>,
    progress: crate::progress::application::ProgressStore,
}

impl ReviewReadiness {
    pub fn new(
        publisher: Arc<dyn ReviewPublisher>,
        scope: ReviewScope,
        host: Option<String>,
        progress: crate::progress::application::ProgressStore,
    ) -> Self {
        Self {
            publisher,
            scope,
            host,
            progress,
        }
    }

    pub fn execute(&self) -> Result<Standing> {
        let scope = self.scope.get()?;
        let branch = scope.branch.clone();
        Ok(Standing {
            host: self.host.clone(),
            readiness: self.publisher.readiness(&branch)?,
            branch,
            read: self.progress.current_paths(&scope.files)?,
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
    pub host: Option<String>,
    pub branch: String,
    pub readiness: Readiness,
    pub read: Vec<String>,
}
