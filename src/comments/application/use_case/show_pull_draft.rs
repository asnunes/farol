use std::sync::Arc;

use crate::comments::domain::{PullDraft, PullDraftStore, Readiness};
use crate::error::Result;

/// What the pull request will say, for the form that opens one.
///
/// Apart from where the review stands, which a failure to read the draft
/// should not take down: the draft only serves the form, and the form only
/// exists while there is a pull request to open.
#[derive(Clone)]
pub struct ShowPullDraft {
    drafts: Arc<dyn PullDraftStore>,
}

impl ShowPullDraft {
    pub fn new(drafts: Arc<dyn PullDraftStore>) -> Self {
        Self { drafts }
    }

    /// Only while there is a pull request to open: once it exists, the draft
    /// has said what it had to.
    pub fn execute(&self, readiness: &Readiness) -> Result<Option<PullDraft>> {
        match readiness {
            Readiness::NoPullRequest { .. } | Readiness::BranchNotPushed => self.drafts.read(),
            _ => Ok(None),
        }
    }
}
