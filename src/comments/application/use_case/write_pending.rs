use std::sync::Arc;

use crate::comments::application::{Comments, PullRequestGate};
use crate::comments::domain::{
    Author, Comment, CommentError, PendingRef, RemoteState, ReviewPublisher,
};
use crate::error::Result;

/// Give a thread its comment for the pull request, in the reviewer's pending
/// review on GitHub — written by them, or by the agent asked to.
///
/// The three ways the screen offers (generate, add, use a turn of the
/// conversation) and `farol pending write` all arrive here, so there is one
/// set of rules for what may go onto the pull request.
///
/// A thread that already has one gets a new one in its place. The new one is
/// added before the old one is taken back, so a failure on the way leaves the
/// thread with a comment rather than without one.
#[derive(Clone)]
pub struct WritePending {
    comments: Comments,
    pull: PullRequestGate,
    publisher: Arc<dyn ReviewPublisher>,
}

impl WritePending {
    pub fn new(
        comments: Comments,
        pull: PullRequestGate,
        publisher: Arc<dyn ReviewPublisher>,
    ) -> Self {
        Self {
            comments,
            pull,
            publisher,
        }
    }

    pub fn execute(&self, id: &str, body: &str, by: Author) -> Result<Comment> {
        let body = body.trim();
        if body.is_empty() {
            return Err(CommentError::Empty.into());
        }
        // Through `live`, which refuses a thread the diff no longer reaches:
        // a line GitHub would refuse is refused here, before the round trip.
        let thread = self.comments.live(id)?;
        let pull = self.pull.at_head()?;
        if let Some(old) = &thread.pending {
            let state = self.publisher.remote(std::slice::from_ref(&old.id))?;
            if matches!(
                state.first().map(|r| &r.state),
                Some(RemoteState::Published { .. })
            ) {
                return Err(CommentError::AlreadyPublished { id: id.into() }.into());
            }
        }

        let lines = thread
            .about
            .lines()
            .ok_or_else(|| CommentError::NotOnCode {
                id: id.into(),
                about: thread.at(),
            })?;
        let added = self
            .publisher
            .add_pending(&pull.id, &pull.head, lines, body)?;
        let written = self
            .comments
            .record_pending(id, PendingRef { id: added, by })?;
        if let Some(old) = thread.pending {
            self.publisher.remove_pending(&old.id)?;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::application::testing::World;
    use crate::comments::domain::Purpose;
    use crate::testing::FakePublisher;

    #[test]
    fn a_pending_comment_goes_onto_the_pull_request_and_its_id_stays_here() {
        let world = World::ready();
        let thread = world.thread();

        let written = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap();

        let pending = written.pending.unwrap();
        assert_eq!(
            world.publisher.state_of(&pending.id),
            RemoteState::Pending {
                body: "Rename it.".into()
            }
        );
        assert_eq!(pending.by, Author::Reviewer);
    }

    #[test]
    fn a_new_one_replaces_the_old_only_once_it_is_there() {
        let world = World::ready();
        let thread = world.thread();
        let write = world.write_pending();
        let first = write
            .execute(&thread.id, "First.", Author::Reviewer)
            .unwrap()
            .pending
            .unwrap();

        let second = write
            .execute(&thread.id, "Second.", Author::Agent("Codex".into()))
            .unwrap()
            .pending
            .unwrap();

        assert_eq!(
            world.publisher.calls(),
            vec![
                format!("add {}", first.id),
                format!("add {}", second.id),
                format!("remove {}", first.id),
            ],
            "the old one is taken back only after the new one exists"
        );
        assert_eq!(world.publisher.state_of(&first.id), RemoteState::Gone);
    }

    #[test]
    fn a_failed_write_keeps_the_comment_already_there() {
        let world = World::with(FakePublisher::ready_at("head").adding_fails("rate limited"));
        let thread = world.thread();
        world
            .comments
            .record_pending(
                &thread.id,
                PendingRef {
                    id: "PRRC_old".into(),
                    by: Author::Reviewer,
                },
            )
            .unwrap();

        assert!(
            world
                .write_pending()
                .execute(&thread.id, "New.", Author::Reviewer)
                .is_err()
        );
        assert_eq!(
            world.comments.get(&thread.id).unwrap().pending.unwrap().id,
            "PRRC_old"
        );
    }

    #[test]
    fn the_agent_writing_it_settles_the_wait_for_it() {
        let world = World::ready();
        let thread = world.thread();
        world
            .comments
            .sent(&thread.id, "Codex", Purpose::Pending)
            .unwrap();

        let written = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Agent("Codex".into()))
            .unwrap();

        assert_eq!(written.conversation.waiting, None);
    }

    #[test]
    fn nothing_goes_onto_a_pull_request_showing_another_commit() {
        let world = World::with(FakePublisher::ready_at("theirs"));
        let thread = world.thread();

        let error = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap_err();

        assert!(error.to_string().contains("pull request is at"), "{error}");
        assert!(world.publisher.calls().is_empty());
    }

    #[test]
    fn a_comment_already_submitted_cannot_be_replaced() {
        let world = World::with(FakePublisher::ready_at("head").holding(
            "PRRC_sent",
            RemoteState::Published {
                body: "Old.".into(),
                url: "https://example.test/1".into(),
            },
        ));
        let thread = world.thread();
        world
            .comments
            .record_pending(
                &thread.id,
                PendingRef {
                    id: "PRRC_sent".into(),
                    by: Author::Reviewer,
                },
            )
            .unwrap();

        let error = world
            .write_pending()
            .execute(&thread.id, "New.", Author::Reviewer)
            .unwrap_err();

        assert!(error.to_string().contains("already submitted"), "{error}");
    }
}
