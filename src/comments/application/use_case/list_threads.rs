use crate::comments::application::Comments;
use crate::comments::domain::Found;
use crate::error::Result;

/// Every thread still waiting for an answer, split by whether the diff still
/// reaches its lines, and the files that could not be read.
#[derive(Clone)]
pub struct ListThreads {
    comments: Comments,
}

impl ListThreads {
    pub fn new(comments: Comments) -> Self {
        Self { comments }
    }

    pub fn execute(&self) -> Result<Found> {
        self.comments.all()
    }
}
