use std::sync::Arc;

use crate::comments::application::PullRequestGate;
use crate::comments::domain::{BranchPusher, CommentError, Drift};
use crate::error::Result;

/// Push the branch under review, so the pull request shows the commit that was
/// read and the read marks can go.
///
/// Only the reviewer's own pull request, and only when the branch is simply
/// ahead of it. Pushing somebody else's branch is not something a review tool
/// should make one click away, and a pull request that is ahead or apart needs
/// a pull, which a push would refuse anyway.
#[derive(Clone)]
pub struct PushBranch {
    pull: PullRequestGate,
    pusher: Arc<dyn BranchPusher>,
}

impl PushBranch {
    pub fn new(pull: PullRequestGate, pusher: Arc<dyn BranchPusher>) -> Self {
        Self { pull, pusher }
    }

    pub fn execute(&self) -> Result<()> {
        let pull = self.pull.open()?;
        if !pull.mine {
            return Err(CommentError::NotYourPull.into());
        }
        let branch = self.pull.branch()?;
        match self.pull.drift(&pull.head)? {
            Some(Drift::Unpushed { .. }) => self.pusher.push(&branch),
            Some(Drift::Uncommitted) => Err(CommentError::Uncommitted.into()),
            // The refusal that says whose commits are missing and what to run.
            Some(Drift::Newer) => self.pull.at_head().map(drop),
            None => Err(CommentError::NothingToPush { branch }.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::PushBranch;
    use crate::comments::application::testing::World;
    use crate::testing::{FakeDiffSource, FakePublisher, RecordingPusher};

    fn push(
        publisher: FakePublisher,
        source: FakeDiffSource,
    ) -> (Arc<RecordingPusher>, crate::error::Result<()>) {
        let world = World::over(publisher, source);
        let pusher = Arc::new(RecordingPusher::default());
        let result = PushBranch::new(world.gate(), pusher.clone()).execute();
        (pusher, result)
    }

    #[test]
    fn your_branch_ahead_of_your_pull_request_is_pushed() {
        let (pusher, result) = push(
            FakePublisher::ready_at("old").mine(),
            FakeDiffSource::with_paths(&["src/a.rs"]).with_ancestors(&["old"]),
        );
        result.unwrap();
        assert_eq!(pusher.pushed(), vec!["feature/x".to_string()]);
    }

    #[test]
    fn somebody_elses_branch_is_never_pushed() {
        let (pusher, result) = push(
            FakePublisher::ready_at("old"),
            FakeDiffSource::with_paths(&["src/a.rs"]).with_ancestors(&["old"]),
        );
        assert!(result.unwrap_err().to_string().contains("only your own"));
        assert!(pusher.pushed().is_empty());
    }

    #[test]
    fn a_pull_request_with_newer_commits_asks_for_a_pull_not_a_push() {
        let (pusher, result) = push(
            FakePublisher::ready_at("newer77").mine(),
            FakeDiffSource::with_paths(&["src/a.rs"]),
        );
        let err = result.unwrap_err().to_string();
        assert!(err.starts_with("Pull first"), "{err}");
        assert!(pusher.pushed().is_empty());
    }
}
