use crate::comments::application::Comments;
use crate::comments::domain::Comment;
use crate::error::Result;

/// The agent saying it has started on a thread, so the reviewer sees it.
#[derive(Clone)]
pub struct AcknowledgeThread {
    comments: Comments,
}

impl AcknowledgeThread {
    pub fn new(comments: Comments) -> Self {
        Self { comments }
    }

    pub fn execute(&self, id: &str, by: &str) -> Result<Comment> {
        self.comments.acknowledge(id, by)
    }
}
