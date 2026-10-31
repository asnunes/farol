use std::sync::Arc;

use crate::comments::application::{Comments, OpenPull, PullRequestGate};
use crate::comments::domain::{CommentError, RemoteState, Review, ReviewPublisher, Verdict};
use crate::diff::application::ReviewScope;
use crate::error::Result;
use crate::shared::short;

/// Submit the review: the verdict and summary, over the pending comments that
/// are already on the pull request.
///
/// The comments went to GitHub one at a time, as they were written, so there
/// is nothing left to assemble and nothing half-sent to recover: this is one
/// call on something that already exists. What stays here is what GitHub
/// would refuse with a bare "Unprocessable Entity", checked first so the
/// reviewer is told which thing they did was the problem.
#[derive(Clone)]
pub struct PublishReview {
    comments: Comments,
    pull: PullRequestGate,
    publisher: Arc<dyn ReviewPublisher>,
    scope: ReviewScope,
    read: crate::lenses::application::ListReadFiles,
}

impl PublishReview {
    pub fn new(
        comments: Comments,
        pull: PullRequestGate,
        publisher: Arc<dyn ReviewPublisher>,
        scope: ReviewScope,
        read: crate::lenses::application::ListReadFiles,
    ) -> Self {
        Self {
            comments,
            pull,
            publisher,
            scope,
            read,
        }
    }

    pub fn execute(&self, verdict: Verdict, summary: &str) -> Result<Sent> {
        let summary = summary.trim();
        if verdict.needs_summary() && summary.is_empty() {
            return Err(CommentError::NoSummary.into());
        }
        let pull = self.pull.open()?;
        if pull.mine && verdict != Verdict::Comment {
            return Err(CommentError::OwnPullRequest.into());
        }
        let comments = self.pending_count()?;

        let url = self.publisher.submit(&Review {
            pull: pull.id.clone(),
            head: pull.head.clone(),
            summary: summary.to_string(),
            verdict,
        })?;

        // After the review, and unable to bring it down. The review is posted
        // and cannot be taken back; the ticks are a convenience, so everything
        // that can go wrong from here is reported beside a review that went.
        let read = self.tick(&pull);
        Ok(Sent {
            url,
            comments,
            read: *read.as_ref().unwrap_or(&0),
            read_failed: read.err().map(|e| e.to_string()),
        })
    }

    /// Mark what was read, when the pull request shows the commit it was read
    /// at. A file read at another commit has not been read in the shape the
    /// pull request shows.
    fn tick(&self, pull: &OpenPull) -> Result<usize> {
        let scope = self.scope.get()?;
        let read = self.already_read()?;
        if read.is_empty() {
            return Ok(0);
        }
        if scope.dirty || scope.head_sha != pull.head {
            return Err(crate::error::Error::msg(format!(
                "the pull request is at {}, you read {}",
                short(&pull.head),
                short(&scope.head_sha)
            )));
        }
        self.publisher.mark_read(&pull.id, &read)
    }

    /// The files the reviewer has read and that have not moved since.
    fn already_read(&self) -> Result<Vec<String>> {
        self.read.execute()
    }

    /// How many of the threads' comments are about to go, as the host has
    /// them.
    fn pending_count(&self) -> Result<usize> {
        let ids: Vec<String> = self
            .comments
            .every()?
            .into_iter()
            .filter_map(|thread| thread.pending.map(|pending| pending.id))
            .collect();
        Ok(self
            .publisher
            .remote(&ids)?
            .iter()
            .filter(|remote| matches!(remote.state, RemoteState::Pending { .. }))
            .count())
    }
}

/// What went, and where it can be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub url: String,
    pub comments: usize,
    /// Files ticked as read on the pull request, so a second round shows what
    /// changed rather than everything.
    pub read: usize,
    /// What stopped the ticks, when something did. The review went either way,
    /// which is why this is a note and not an error.
    pub read_failed: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::application::testing::World;
    use crate::comments::domain::{Author, Readiness};
    use crate::progress::application::MarkViewed;
    use crate::testing::FakePublisher;

    #[test]
    fn the_pending_comments_go_with_the_verdict_and_are_published_on_the_host() {
        let world = World::ready();
        let thread = world.thread();
        let pending = world
            .write_pending()
            .execute(&thread.id, "Rename it.", Author::Reviewer)
            .unwrap()
            .pending
            .unwrap();

        let sent = world
            .publish_review()
            .execute(Verdict::Comment, "")
            .unwrap();

        assert_eq!(sent.comments, 1);
        assert_eq!(world.publisher.sent().pull, "PR_kwDO");
        assert!(matches!(
            world.publisher.state_of(&pending.id),
            RemoteState::Published { .. }
        ));
    }

    #[test]
    fn asking_for_something_without_saying_what_is_refused_before_anything_is_sent() {
        let world = World::ready();

        assert!(
            world
                .publish_review()
                .execute(Verdict::RequestChanges, "  \n ")
                .is_err()
        );
        assert!(world.publisher.nothing_sent());
    }

    #[test]
    fn a_verdict_nobody_can_give_on_their_own_pull_request_is_refused_here() {
        // GitHub answers a bare "Unprocessable Entity" for this, which leaves
        // the reviewer guessing which of the things they just did was wrong.
        let world = World::with(FakePublisher::ready_at("head").mine());

        let err = world
            .publish_review()
            .execute(Verdict::Approve, "reads well")
            .unwrap_err();

        assert!(err.to_string().contains("yours"), "{err}");
        assert!(world.publisher.nothing_sent());
        world
            .publish_review()
            .execute(Verdict::Comment, "worth a note")
            .unwrap();
    }

    #[test]
    fn a_pull_request_that_moved_on_still_takes_the_review_but_not_the_ticks() {
        // The comments are anchored where they were written; the verdict is
        // about the pull request. Only the ticks are about the commit read.
        let world = World::with(FakePublisher::ready_at("theirs"));
        MarkViewed::new(world.progress.clone())
            .execute("src/a.rs", "now")
            .unwrap();

        let sent = world
            .publish_review()
            .execute(Verdict::Approve, "")
            .unwrap();

        assert_eq!(sent.url, "https://example.test/r1");
        assert_eq!(sent.read, 0);
        assert!(sent.read_failed.unwrap().contains("pull request is at"));
        assert!(world.publisher.marked().is_empty());
    }

    #[test]
    fn the_files_already_read_go_up_ticked_with_the_review() {
        let world = World::ready();
        MarkViewed::new(world.progress.clone())
            .execute("src/a.rs", "now")
            .unwrap();

        let sent = world
            .publish_review()
            .execute(Verdict::Approve, "")
            .unwrap();

        assert_eq!(world.publisher.marked(), vec!["src/a.rs".to_string()]);
        assert_eq!(sent.read, 1);
    }

    #[test]
    fn a_host_that_will_not_tick_does_not_undo_a_review_that_went() {
        let world = World::with(FakePublisher::ready_at("head").marking_fails("no permission"));
        MarkViewed::new(world.progress.clone())
            .execute("src/a.rs", "now")
            .unwrap();

        let sent = world
            .publish_review()
            .execute(Verdict::Approve, "")
            .unwrap();

        assert_eq!(sent.url, "https://example.test/r1");
        assert!(
            sent.read_failed.unwrap().contains("no permission"),
            "the reviewer has to be told which half did not happen"
        );
    }

    #[test]
    fn every_way_of_not_being_ready_is_refused_in_its_own_words() {
        for (readiness, expected) in [
            (Readiness::NoRemote, "no remote"),
            (Readiness::NoToken, "no token"),
            (Readiness::TokenRefused, "would not answer with this token"),
            (Readiness::BranchNotPushed, "not on GitHub yet"),
            (
                Readiness::NoPullRequest {
                    open_at: "https://example.test/compare".into(),
                },
                "no pull request yet",
            ),
            (
                Readiness::Ambiguous {
                    numbers: vec![4, 7],
                },
                "#4, #7",
            ),
        ] {
            let world = World::with(FakePublisher::blocked(readiness));

            let err = world
                .publish_review()
                .execute(Verdict::Approve, "")
                .unwrap_err()
                .to_string();

            assert!(err.contains(expected), "{err}");
            assert!(world.publisher.nothing_sent());
        }
    }
}
