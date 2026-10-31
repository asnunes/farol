use std::sync::Arc;

use crate::comments::domain::{CommentError, RemoteState, ReviewPublisher};
use crate::error::Result;

/// Change the words of a pending comment, where it sits on GitHub.
///
/// Kept apart from writing a new one because it keeps the comment GitHub
/// already has, which is what an edit is on the pull request's page too. It
/// goes by GitHub's id for the comment, which every pending comment has, with
/// a thread or without one.
#[derive(Clone)]
pub struct EditPending {
    publisher: Arc<dyn ReviewPublisher>,
}

impl EditPending {
    pub fn new(publisher: Arc<dyn ReviewPublisher>) -> Self {
        Self { publisher }
    }

    pub fn execute(&self, id: &str, body: &str) -> Result<()> {
        let body = body.trim();
        if body.is_empty() {
            return Err(CommentError::Empty.into());
        }
        match self
            .publisher
            .remote(&[id.to_string()])?
            .first()
            .map(|remote| &remote.state)
        {
            Some(RemoteState::Pending { .. }) => self.publisher.edit_pending(id, body),
            // A submitted comment is the pull request's now.
            Some(RemoteState::Published { .. }) => {
                Err(CommentError::AlreadyPublished { id: id.into() }.into())
            }
            _ => Err(CommentError::NoPending { id: id.into() }.into()),
        }
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
            .execute(&pending.id, "Rename it, please.")
            .unwrap();

        assert_eq!(
            world.publisher.state_of(&pending.id),
            RemoteState::Pending {
                body: "Rename it, please.".into()
            }
        );
    }

    #[test]
    fn a_comment_not_in_the_pending_review_has_nothing_to_edit() {
        let world = World::ready();

        let error = world
            .edit_pending()
            .execute("PRRC_unknown", "Hi.")
            .unwrap_err();

        assert!(
            error.to_string().contains("not a pending comment"),
            "{error}"
        );
    }
}
