use crate::comments::application::Comments;
use crate::error::Result;

/// Close a thread, which is to say drop it.
///
/// Its pending comment stays on the pull request. The thread is the
/// conversation that led to it, and ending the conversation is not taking back
/// what the reviewer means to publish: the comment is still drawn on its lines,
/// read from the pending review, and removing it is an act of its own.
#[derive(Clone)]
pub struct CloseThread {
    comments: Comments,
}

impl CloseThread {
    pub fn new(comments: Comments) -> Self {
        Self { comments }
    }

    pub fn execute(&self, id: &str) -> Result<()> {
        self.comments.close(id)
    }
}

#[cfg(test)]
mod tests {
    use crate::comments::application::testing::World;
    use crate::comments::domain::{Author, RemoteState};

    #[test]
    fn closing_a_thread_leaves_its_pending_comment_on_the_pull_request() {
        let world = World::ready();
        let thread = world.thread();
        let pending = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap()
            .pending
            .unwrap();

        world.close_thread().execute(&thread.id).unwrap();

        assert_eq!(
            world.publisher.state_of(&pending.id),
            RemoteState::Pending {
                body: "Rename it.".into()
            },
            "the comment is what the reviewer means to publish; the thread only led to it"
        );
        assert!(world.comments.all().unwrap().comments.is_empty());
    }
}
