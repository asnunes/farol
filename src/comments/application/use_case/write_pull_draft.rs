use std::sync::Arc;

use crate::comments::domain::{CommentError, PullDraft, PullDraftStore};
use crate::error::Result;

/// Keep what the pull request will say, for the page to pick up. Written by
/// the agent after Generate, through `farol github draft`.
#[derive(Clone)]
pub struct WritePullDraft {
    drafts: Arc<dyn PullDraftStore>,
}

impl WritePullDraft {
    pub fn new(drafts: Arc<dyn PullDraftStore>) -> Self {
        Self { drafts }
    }

    pub fn execute(&self, title: &str, body: &str) -> Result<PullDraft> {
        let draft = PullDraft {
            title: title.trim().to_string(),
            body: body.trim().to_string(),
        };
        if draft.title.is_empty() {
            return Err(CommentError::NoTitle.into());
        }
        self.drafts.write(&draft)?;
        Ok(draft)
    }
}
