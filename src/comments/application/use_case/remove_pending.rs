use std::sync::Arc;

use crate::comments::application::Comments;
use crate::comments::domain::{CommentError, RemoteState, ReviewPublisher};
use crate::error::Result;

/// Take a pending comment back off the pull request. A thread pointing at it
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
    /// `id` is GitHub's id for the comment. A thread that points at it forgets
    /// it; one that was closed, or never existed, has nothing to forget.
    pub fn execute(&self, id: &str) -> Result<()> {
        let state = self.publisher.remote(&[id.to_string()])?;
        if let Some(RemoteState::Published { .. }) = state.first().map(|remote| &remote.state) {
            return Err(CommentError::AlreadyPublished { id: id.into() }.into());
        }
        self.publisher.remove_pending(id)?;
        let pointing = self
            .comments
            .every()?
            .into_iter()
            .find(|thread| thread.pending.as_ref().is_some_and(|p| p.id == id));
        if let Some(thread) = pointing {
            self.comments.clear_pending(&thread.id)?;
        }
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

        world.remove_pending().execute(&pending.id).unwrap();

        assert_eq!(world.publisher.state_of(&pending.id), RemoteState::Gone);
        assert_eq!(world.comments.get(&thread.id).unwrap().pending, None);
    }

    #[test]
    fn a_comment_no_thread_points_at_is_removed_by_its_own_id() {
        let world = World::ready();
        let thread = world.thread();
        let pending = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap()
            .pending
            .unwrap();
        world.close_thread().execute(&thread.id).unwrap();

        world.remove_pending().execute(&pending.id).unwrap();

        assert_eq!(world.publisher.state_of(&pending.id), RemoteState::Gone);
    }
}
