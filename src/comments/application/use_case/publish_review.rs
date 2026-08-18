use std::sync::Arc;

use crate::comments::domain::{
    Comment, CommentError, CommentStore, Readiness, Review, ReviewPublisher, Verdict,
};
use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
use crate::error::Result;

/// Send the review to the pull request: the summary, the verdict, and every
/// comment that has not gone yet.
///
/// Everything here is a refusal except the last two lines. That is the shape of
/// the operation: publishing is irreversible in the way that matters — a
/// comment posted against the wrong commit lands on unrelated code, quietly,
/// under the reviewer's name — so each way that could happen is closed before
/// anything is sent.
pub struct PublishReview {
    store: Arc<dyn CommentStore>,
    publisher: Arc<dyn ReviewPublisher>,
    scope: ReviewScope,
    diffs: FileDiffs,
    history: CommitHistory,
}

impl PublishReview {
    pub fn new(
        store: Arc<dyn CommentStore>,
        publisher: Arc<dyn ReviewPublisher>,
        scope: ReviewScope,
        diffs: FileDiffs,
        history: CommitHistory,
    ) -> Self {
        Self {
            store,
            publisher,
            scope,
            diffs,
            history,
        }
    }

    pub fn execute(&self, verdict: Verdict, summary: &str) -> Result<Sent> {
        let summary = summary.trim();
        if verdict.needs_summary() && summary.is_empty() {
            return Err(CommentError::NoSummary.into());
        }

        let scope = self.scope.get()?;
        if scope.dirty {
            return Err(CommentError::Uncommitted.into());
        }

        let (pull_request, theirs) = match self.publisher.readiness(&scope.branch)? {
            Readiness::Ready { pull_request, head } => (pull_request, head),
            Readiness::NoRemote => return Err(CommentError::NoRemote.into()),
            Readiness::NoToken => return Err(CommentError::NoToken.into()),
            Readiness::TokenRefused => return Err(CommentError::TokenRefused.into()),
            Readiness::BranchNotPushed => {
                return Err(CommentError::BranchNotPushed {
                    branch: scope.branch.clone(),
                }
                .into());
            }
            Readiness::NoPullRequest { .. } => {
                return Err(CommentError::NoPullRequest {
                    branch: scope.branch.clone(),
                }
                .into());
            }
        };

        if theirs != scope.head_sha {
            return Err(CommentError::HeadMoved {
                theirs: short(&theirs),
                ours: short(&scope.head_sha),
                behind: !self.history.is_ancestor(&theirs)?,
            }
            .into());
        }

        let waiting: Vec<Comment> = self
            .store
            .list()?
            .comments
            .into_iter()
            .filter(|c| c.published.is_none())
            .collect();
        self.still_in_the_diff(&waiting)?;

        let url = self.publisher.publish(&Review {
            pull_request,
            head: scope.head_sha.clone(),
            summary: summary.to_string(),
            verdict,
            comments: waiting.clone(),
        })?;

        for comment in &waiting {
            self.store.save(&Comment {
                published: Some(url.clone()),
                ..comment.clone()
            })?;
        }
        Ok(Sent {
            url,
            comments: waiting.len(),
        })
    }

    /// The same rule `Comments::add` applies, asked again at the last moment.
    ///
    /// Between writing a comment and sending it the reviewer may have pulled,
    /// and the diff is not the one the comment was written against any more.
    /// The ones that drifted are named together rather than one at a time: the
    /// reviewer has to go and look at each, and being sent back for the next
    /// one after every fix is the worse version of the same information.
    fn still_in_the_diff(&self, comments: &[Comment]) -> Result<()> {
        let scope = self.scope.get()?;
        let mut drifted = Vec::new();
        for comment in comments {
            let shown = scope.contains(&comment.path)
                && self
                    .diffs
                    .of(&comment.path)?
                    .shows(comment.from, comment.to);
            if !shown {
                drifted.push(comment.at());
            }
        }
        match drifted.is_empty() {
            true => Ok(()),
            false => Err(CommentError::NoLongerInDiff { comments: drifted }.into()),
        }
    }
}

/// What went, and where it can be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub url: String,
    pub comments: usize,
}

/// Shas are compared in full and shown short: nobody reads forty characters,
/// and seven is what every other tool prints.
fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::domain::Hunk;
    use crate::testing::{FakeDiffSource, FakePublisher, InMemoryComments};

    #[test]
    fn a_review_goes_out_with_the_commit_it_was_read_at() {
        // The whole point of sending the sha: the far end anchors the comments
        // against it rather than against whatever is newest.
        let (publisher, publish) = published();

        let sent = publish.execute(Verdict::Approve, "").unwrap();

        let review = publisher.sent();
        assert_eq!(review.head, "head");
        assert_eq!(review.pull_request, 12);
        assert_eq!(review.verdict, Verdict::Approve);
        assert_eq!(sent.comments, 0);
    }

    #[test]
    fn asking_for_something_without_saying_what_is_refused_before_anything_is_sent() {
        // Refused here rather than at the far end, where the round trip would
        // be spent to be told so and the reviewer would have to write it again.
        let (publisher, publish) = published();

        assert!(publish.execute(Verdict::RequestChanges, "  \n ").is_err());
        assert!(publisher.nothing_sent());
    }

    #[test]
    fn a_comment_verdict_goes_out_on_the_comments_alone() {
        // The line comments are the review. farol drafts them and submits the
        // verdict at the draft, which is the road that lets the summary be
        // empty; demanding one here would be farol's own rule and not the
        // host's.
        let (publisher, publish) = published();
        publish.store.save(&comment("1", 10, 12)).unwrap();

        publish.execute(Verdict::Comment, "").unwrap();

        assert_eq!(publisher.sent().comments.len(), 1);
    }

    #[test]
    fn a_comment_that_has_gone_once_does_not_go_again() {
        let (publisher, publish) = published();
        publish.store.save(&comment("1", 10, 12)).unwrap();
        publish
            .store
            .save(&Comment {
                published: Some("https://example.test/1".into()),
                ..comment("2", 11, 11)
            })
            .unwrap();

        let sent = publish.execute(Verdict::Comment, "Reads well.").unwrap();

        assert_eq!(sent.comments, 1);
        let review = publisher.sent();
        assert_eq!(review.comments.len(), 1);
        assert_eq!(review.comments[0].id, "1");
    }

    #[test]
    fn what_went_comes_back_marked_with_where_it_landed() {
        // Marked and not removed: a published comment is still a question, and
        // the answer is going to arrive on the pull request.
        let (_publisher, publish) = published();
        publish.store.save(&comment("1", 10, 12)).unwrap();

        publish.execute(Verdict::Comment, "Reads well.").unwrap();

        let all = publish.store.list().unwrap().comments;
        assert_eq!(all.len(), 1, "publishing is not closing");
        assert_eq!(all[0].published.as_deref(), Some("https://example.test/r1"));
    }

    #[test]
    fn comments_the_diff_no_longer_shows_are_named_and_nothing_is_sent() {
        // The branch moved under them between writing and sending. All of them
        // at once: being sent back for the next one after each fix is the same
        // information delivered worse.
        let (publisher, publish) = published();
        publish.store.save(&comment("1", 10, 12)).unwrap();
        publish.store.save(&comment("2", 30, 30)).unwrap();
        publish.store.save(&comment("3", 33, 33)).unwrap();

        let err = publish
            .execute(Verdict::Comment, "Reads well.")
            .unwrap_err()
            .to_string();

        assert!(err.contains("src/a.rs:30"), "{err}");
        assert!(err.contains("src/a.rs:33"), "{err}");
        assert!(!err.contains("src/a.rs:10-12"), "{err}");
        assert!(publisher.nothing_sent(), "nothing goes until all of it can");
    }

    #[test]
    fn a_pull_request_showing_another_commit_is_refused_and_says_which_way() {
        let publisher = Arc::new(FakePublisher::ready_at("older99"));
        let publish = publish_with(publisher.clone(), &["older99"]);

        let err = publish
            .execute(Verdict::Approve, "")
            .unwrap_err()
            .to_string();

        assert!(err.contains("older9"), "{err}");
        assert!(err.contains("head"), "{err}");
        assert!(err.contains("Push first."), "{err}");
        assert!(publisher.nothing_sent());
    }

    #[test]
    fn a_pull_request_ahead_of_us_says_to_pull_instead() {
        // Their commit is not in our history, so the missing half is ours to
        // fetch. Told "push" here, the reviewer would push nothing and be stuck.
        let publisher = Arc::new(FakePublisher::ready_at("newer77"));
        let publish = publish_with(publisher.clone(), &[]);

        let err = publish
            .execute(Verdict::Approve, "")
            .unwrap_err()
            .to_string();

        assert!(err.contains("Pull first."), "{err}");
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
        ] {
            let publisher = Arc::new(FakePublisher::blocked(readiness));
            let publish = publish_with(publisher.clone(), &[]);

            let err = publish
                .execute(Verdict::Approve, "")
                .unwrap_err()
                .to_string();

            assert!(err.contains(expected), "{err}");
            assert!(publisher.nothing_sent());
        }
    }

    #[test]
    fn a_review_of_uncommitted_work_cannot_be_published_at_all() {
        // There is no commit to anchor to: `--dirty` reviews the working tree,
        // and the pull request has never seen it.
        let publisher = Arc::new(FakePublisher::ready_at("head"));
        let source = Arc::new(
            FakeDiffSource::with_paths(&["src/a.rs"])
                .with_line_count("src/a.rs", 200)
                .dirty(),
        );
        let publish = over(publisher.clone(), source);

        let err = publish
            .execute(Verdict::Approve, "")
            .unwrap_err()
            .to_string();

        assert!(err.contains("uncommitted"), "{err}");
        assert!(publisher.nothing_sent());
    }

    /// A review of one file whose diff prints lines 10–14 and 40–42, with an
    /// open pull request sitting on the commit we are on.
    fn published() -> (Arc<FakePublisher>, PublishReview) {
        let publisher = Arc::new(FakePublisher::ready_at("head"));
        (publisher.clone(), publish_with(publisher, &[]))
    }

    fn publish_with(publisher: Arc<FakePublisher>, ancestors: &[&str]) -> PublishReview {
        let source = Arc::new(
            FakeDiffSource::with_paths(&["src/a.rs"])
                .with_line_count("src/a.rs", 200)
                .showing("src/a.rs", vec![hunk(10, 5), hunk(40, 3)])
                .with_ancestors(ancestors),
        );
        over(publisher, source)
    }

    fn over(publisher: Arc<FakePublisher>, source: Arc<FakeDiffSource>) -> PublishReview {
        PublishReview::new(
            Arc::new(InMemoryComments::default()),
            publisher,
            ReviewScope::new(source.clone()),
            FileDiffs::new(source.clone()),
            CommitHistory::new(source),
        )
    }

    fn hunk(start: u32, lines: u32) -> Hunk {
        Hunk {
            old_start: start,
            old_lines: lines,
            new_start: start,
            new_lines: lines,
            lines: vec![],
        }
    }

    fn comment(id: &str, from: u32, to: u32) -> Comment {
        Comment {
            id: id.into(),
            path: "src/a.rs".into(),
            from,
            to,
            body: "Why this order?".into(),
            published: None,
        }
    }
}
