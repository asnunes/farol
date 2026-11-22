use std::collections::BTreeSet;
use std::sync::Arc;

use crate::comments::application::PullRequestGate;
use crate::comments::domain::{ReadFiles, ReadMarks};
use crate::error::Result;

/// Whether the pull request's read marks already say what farol's do: the
/// question the sync button answers before it is pressed. Writes nothing.
#[derive(Clone)]
pub struct CheckReadMarks {
    pull: PullRequestGate,
    publisher: Arc<dyn ReadMarks>,
    read: Arc<dyn ReadFiles>,
}

impl CheckReadMarks {
    pub fn new(
        pull: PullRequestGate,
        publisher: Arc<dyn ReadMarks>,
        read: Arc<dyn ReadFiles>,
    ) -> Self {
        Self {
            pull,
            publisher,
            read,
        }
    }

    /// The same sets a sync would make equal, compared. Refused, as a sync is,
    /// when the pull request shows another commit.
    pub fn execute(&self) -> Result<bool> {
        let pull = self.pull.at_head()?;
        let read: BTreeSet<String> = self.read.read()?.into_iter().collect();
        let viewed: BTreeSet<String> = self.publisher.viewed(&pull.id)?.into_iter().collect();
        Ok(read == viewed)
    }
}

#[cfg(test)]
mod tests {
    use crate::comments::application::testing::World;
    use crate::progress::application::MarkViewed;
    use crate::testing::FakePublisher;

    #[test]
    fn the_marks_are_in_sync_after_a_sync_and_out_of_it_once_a_file_is_read() {
        let world = World::with(FakePublisher::ready_at("head").mine());
        assert!(
            world.check_read_marks().execute().unwrap(),
            "nothing read, nothing marked"
        );

        MarkViewed::new(world.progress.clone())
            .execute("src/a.rs", "now")
            .unwrap();
        assert!(
            !world.check_read_marks().execute().unwrap(),
            "read here and not on the pull request"
        );

        world.sync_read_marks().execute().unwrap();
        assert!(world.check_read_marks().execute().unwrap());
    }
}
