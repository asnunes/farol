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
        // Over the pull request's files: one the review has and the pull
        // request does not can never be marked, and must not keep the two
        // looking out of sync forever.
        Ok(self
            .publisher
            .files(&pull.id)?
            .iter()
            .all(|file| file.viewed == read.contains(&file.path)))
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

    #[test]
    fn a_file_read_here_that_the_pull_request_does_not_list_leaves_the_marks_in_sync() {
        // A deletion here can be a rename on GitHub: the path read here is one
        // no mark can ever be put on, and the button stayed out of sync for it.
        struct Read(Vec<String>);
        impl crate::comments::domain::ReadFiles for Read {
            fn read(&self) -> crate::error::Result<Vec<String>> {
                Ok(self.0.clone())
            }
        }
        let world = World::with(
            FakePublisher::ready_at("head")
                .mine()
                .viewing(&["src/a.rs"]),
        );
        let check = super::CheckReadMarks::new(
            world.gate(),
            world.publisher.clone(),
            std::sync::Arc::new(Read(vec!["src/a.rs".into(), "src/gone.rs".into()])),
        );

        assert!(check.execute().unwrap());
    }
}
