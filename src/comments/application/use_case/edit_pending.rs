use std::sync::Arc;

use crate::comments::application::Comments;
use crate::comments::domain::{CommentError, PendingRef, RemoteState, ReviewPublisher};
use crate::error::Result;

/// Change the words of a thread's pending comment, where it sits on GitHub.
///
/// Kept apart from writing a new one because it keeps the comment GitHub
/// already has, which is what an edit is on the pull request's page too.
#[derive(Clone)]
pub struct EditPending {
    comments: Comments,
    publisher: Arc<dyn ReviewPublisher>,
}

impl EditPending {
    pub fn new(comments: Comments, publisher: Arc<dyn ReviewPublisher>) -> Self {
        Self {
            comments,
            publisher,
        }
    }

    pub fn execute(&self, id: &str, body: &str) -> Result<()> {
        let body = body.trim();
        if body.is_empty() {
            return Err(CommentError::Empty.into());
        }
        let pending = still_pending(&self.comments, self.publisher.as_ref(), id)?;
        self.publisher.edit_pending(&pending.id, body)
    }
}

/// The thread's pending comment, refused when it has none or when it already
/// went with a review — a submitted comment is the pull request's now.
pub(super) fn still_pending(
    comments: &Comments,
    publisher: &dyn ReviewPublisher,
    id: &str,
) -> Result<PendingRef> {
    let pending = comments
        .get(id)?
        .pending
        .ok_or_else(|| CommentError::NoPending { id: id.into() })?;
    match publisher
        .remote(std::slice::from_ref(&pending.id))?
        .first()
        .map(|remote| &remote.state)
    {
        Some(RemoteState::Pending { .. }) => Ok(pending),
        Some(RemoteState::Published { .. }) => {
            Err(CommentError::AlreadyPublished { id: id.into() }.into())
        }
        _ => Err(CommentError::NoPending { id: id.into() }.into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::comments::application::testing::World;
    use crate::comments::domain::{Author, RemoteState};

    #[test]
    fn an_edit_changes_the_comment_github_already_has() {
        let world = World::ready();
        let thread = world.thread();
        let pending = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap()
            .pending
            .unwrap();

        world
            .edit_pending()
            .execute(&thread.id, "Rename it, please.")
            .unwrap();

        assert_eq!(
            world.publisher.state_of(&pending.id),
            RemoteState::Pending {
                body: "Rename it, please.".into()
            }
        );
    }

    #[test]
    fn a_thread_with_no_pending_comment_has_nothing_to_edit() {
        let world = World::ready();
        let thread = world.thread();

        let error = world.edit_pending().execute(&thread.id, "Hi.").unwrap_err();

        assert!(error.to_string().contains("no pending comment"), "{error}");
    }
}
