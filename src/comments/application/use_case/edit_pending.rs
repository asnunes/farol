use std::sync::Arc;

use crate::comments::application::Comments;
use crate::comments::domain::{CommentError, RemoteState, ReviewPublisher};
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
) -> Result<PendingOf> {
    let pending = pending_named(comments, id)?;
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

/// The pending comment `id` names: a thread's, by the thread's id, or one no
/// thread points at, by the host's id for it. The second is what is left when
/// the comment was written on the pull request's page, or outlived its thread.
pub(super) fn pending_named(comments: &Comments, id: &str) -> Result<PendingOf> {
    let threads = comments.every()?;
    if let Some(thread) = threads.iter().find(|t| t.id == id) {
        let pending = thread
            .pending
            .as_ref()
            .ok_or_else(|| CommentError::NoPending { id: id.into() })?;
        return Ok(PendingOf {
            id: pending.id.clone(),
            thread: Some(thread.id.clone()),
        });
    }
    Ok(PendingOf {
        id: id.to_string(),
        thread: threads
            .iter()
            .find(|t| t.pending.as_ref().is_some_and(|p| p.id == id))
            .map(|t| t.id.clone()),
    })
}

/// A pending comment, and the thread that points at it if one does.
pub(super) struct PendingOf {
    pub id: String,
    pub thread: Option<String>,
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
