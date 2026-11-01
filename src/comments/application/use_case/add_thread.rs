use crate::comments::application::Comments;
use crate::comments::domain::{Comment, Subject};
use crate::error::Result;

/// Start a thread on anything a reviewer can talk about: lines of the diff, a
/// lens, or a block's description.
///
/// The one way in for the terminal and the browser alike. They used to reach
/// two different methods, one for lines and one for the rest, which is the
/// drift a single use case exists to rule out.
#[derive(Clone)]
pub struct AddThread {
    comments: Comments,
}

impl AddThread {
    pub fn new(comments: Comments) -> Self {
        Self { comments }
    }

    pub fn execute(&self, about: Subject, body: &str) -> Result<Comment> {
        self.comments.add(about, body)
    }
}
