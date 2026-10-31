use std::sync::Arc;

use crate::comments::application::PullRequestGate;
use crate::comments::domain::ReviewPublisher;
use crate::error::Result;

/// Make the pull request's read marks say what farol's do: every file read
/// here marked there, and every file marked there but not read here unmarked.
///
/// Its own operation, apart from submitting a review, because a review is not
/// always there to carry it: on your own pull request there is nothing to
/// submit, and a reader with no comment to leave still wants the next round on
/// GitHub to show what changed since they read it.
#[derive(Clone)]
pub struct SyncReadMarks {
    pull: PullRequestGate,
    publisher: Arc<dyn ReviewPublisher>,
    read: crate::lenses::application::ListReadFiles,
}

impl SyncReadMarks {
    pub fn new(
        pull: PullRequestGate,
        publisher: Arc<dyn ReviewPublisher>,
        read: crate::lenses::application::ListReadFiles,
    ) -> Self {
        Self {
            pull,
            publisher,
            read,
        }
    }

    /// Refused when the pull request shows another commit: a file read at one
    /// commit has not been read in the shape the pull request shows.
    pub fn execute(&self) -> Result<Synced> {
        let pull = self.pull.at_head()?;
        let read = self.read.execute()?;
        let stale: Vec<String> = self
            .publisher
            .viewed(&pull.id)?
            .into_iter()
            .filter(|path| !read.contains(path))
            .collect();
        Ok(Synced {
            number: pull.number,
            marked: self.publisher.mark_read(&pull.id, &read)?,
            unmarked: self.publisher.mark_unread(&pull.id, &stale)?,
        })
    }
}

/// What the sync came to on the pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Synced {
    pub number: u32,
    pub marked: usize,
    pub unmarked: usize,
}

#[cfg(test)]
mod tests {
    use crate::comments::application::testing::World;
    use crate::progress::application::MarkViewed;
    use crate::testing::{FakeDiffSource, FakePublisher};

    #[test]
    fn what_was_read_here_is_marked_there_and_nothing_else_stays_marked() {
        let world = World::over(
            FakePublisher::ready_at("head")
                .mine()
                .viewing(&["src/b.rs"]),
            FakeDiffSource::with_paths(&["src/a.rs", "src/b.rs"]),
        );
        MarkViewed::new(world.progress.clone())
            .execute("src/a.rs", "now")
            .unwrap();

        let synced = world.sync_read_marks().execute().unwrap();

        assert_eq!((synced.marked, synced.unmarked), (1, 1));
        assert_eq!(
            world.publisher.marked(),
            vec!["src/a.rs".to_string()],
            "b.rs was unread here, so the pull request stops saying it was read"
        );
        assert!(world.publisher.nothing_sent(), "no review was asked for");
    }

    #[test]
    fn a_sync_refuses_uncommitted_work_and_another_commit() {
        let dirty = World::over(
            FakePublisher::ready_at("head"),
            FakeDiffSource::with_paths(&["src/a.rs"]).dirty(),
        );
        let err = dirty.sync_read_marks().execute().unwrap_err().to_string();
        assert!(err.contains("uncommitted"), "{err}");

        let moved = World::with(FakePublisher::ready_at("newer77"));
        let err = moved.sync_read_marks().execute().unwrap_err().to_string();
        assert!(err.starts_with("Pull first"), "{err}");
    }
}
