use crate::comments::application::Comments;
use crate::comments::domain::{Author, Comment};
use crate::error::Result;

/// Another turn in a thread's conversation, by the reviewer or the agent.
#[derive(Clone)]
pub struct ReplyToThread {
    comments: Comments,
}

impl ReplyToThread {
    pub fn new(comments: Comments) -> Self {
        Self { comments }
    }

    pub fn execute(&self, id: &str, from: Author, body: &str) -> Result<Comment> {
        self.comments.reply(id, from, body)
    }
}
