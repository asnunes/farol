use std::sync::Arc;

use crate::comments::application::Comments;
use crate::comments::domain::{CommentError, RemoteState, ReviewPublisher};
use crate::error::Result;

/// Close a thread, which is to say drop it.
///
/// A thread whose pending comment is still on the pull request is not closed
/// on its own: the comment would stay in the reviewer's pending review with
/// nothing here pointing at it. The screen asks first and then says to take
/// it back too; the terminal refuses and says how.
#[derive(Clone)]
pub struct CloseThread {
    comments: Comments,
    publisher: Arc<dyn ReviewPublisher>,
}

impl CloseThread {
    pub fn new(comments: Comments, publisher: Arc<dyn ReviewPublisher>) -> Self {
        Self {
            comments,
            publisher,
        }
    }

    pub fn execute(&self, id: &str, discard_pending: bool) -> Result<()> {
        if let Some(pending) = self.comments.get(id)?.pending {
            let state = self.publisher.remote(std::slice::from_ref(&pending.id))?;
            if let Some(RemoteState::Pending { .. }) = state.first().map(|remote| &remote.state) {
                if !discard_pending {
                    return Err(CommentError::PendingStillOpen { id: id.into() }.into());
                }
                self.publisher.remove_pending(&pending.id)?;
            }
        }
        self.comments.close(id)
    }
}

#[cfg(test)]
mod tests {
    use crate::comments::application::testing::World;
    use crate::comments::domain::{Author, RemoteState};

    #[test]
    fn a_thread_with_a_pending_comment_is_closed_only_when_told_to_take_it_back() {
        let world = World::ready();
        let thread = world.thread();
        let pending = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap()
            .pending
            .unwrap();

        let error = world.close_thread().execute(&thread.id, false).unwrap_err();
        assert!(
            error.to_string().contains("farol pending remove"),
            "{error}"
        );

        world.close_thread().execute(&thread.id, true).unwrap();
        assert_eq!(world.publisher.state_of(&pending.id), RemoteState::Gone);
        assert!(world.comments.all().unwrap().comments.is_empty());
    }

    #[test]
    fn a_thread_whose_comment_went_with_the_review_closes_like_any_other() {
        let world = World::ready();
        let thread = world.thread();
        world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap();
        world
            .publish_review()
            .execute(crate::comments::domain::Verdict::Comment, "")
            .unwrap();

        world.close_thread().execute(&thread.id, false).unwrap();
    }
}
