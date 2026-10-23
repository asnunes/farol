use std::sync::Arc;

use crate::comments::application::Comments;
use crate::comments::domain::{CommentError, RemoteState, ReviewPublisher};
use crate::error::Result;

/// Take a thread's pending comment back off the pull request. The thread
/// stays: the conversation was never on GitHub.
#[derive(Clone)]
pub struct RemovePending {
    comments: Comments,
    publisher: Arc<dyn ReviewPublisher>,
}

impl RemovePending {
    pub fn new(comments: Comments, publisher: Arc<dyn ReviewPublisher>) -> Self {
        Self {
            comments,
            publisher,
        }
    }

    /// One already deleted on the pull request's page is simply forgotten
    /// here; one already submitted is refused, because removing it from the
    /// thread would not remove it from the pull request.
    pub fn execute(&self, id: &str) -> Result<()> {
        let pending = self
            .comments
            .get(id)?
            .pending
            .ok_or_else(|| CommentError::NoPending { id: id.into() })?;
        let state = self.publisher.remote(std::slice::from_ref(&pending.id))?;
        if let Some(RemoteState::Published { .. }) = state.first().map(|remote| &remote.state) {
            return Err(CommentError::AlreadyPublished { id: id.into() }.into());
        }
        self.publisher.remove_pending(&pending.id)?;
        self.comments.clear_pending(id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::comments::application::testing::World;
    use crate::comments::domain::{Author, RemoteState};

    #[test]
    fn removing_takes_it_off_the_pull_request_and_keeps_the_thread() {
        let world = World::ready();
        let thread = world.thread();
        let pending = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap()
            .pending
            .unwrap();

        world.remove_pending().execute(&thread.id).unwrap();

        assert_eq!(world.publisher.state_of(&pending.id), RemoteState::Gone);
        assert_eq!(world.comments.get(&thread.id).unwrap().pending, None);
    }
}
