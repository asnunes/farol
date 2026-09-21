use std::sync::Arc;

use crate::comments::application::reached;
use crate::comments::domain::{
    Comment, CommentError, CommentStore, Readiness, Review, ReviewPublisher, Verdict,
};
use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
use crate::error::Result;
use crate::progress::application::ProgressStore;
use crate::shared::short;

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
    progress: ProgressStore,
}

impl PublishReview {
    pub fn new(
        store: Arc<dyn CommentStore>,
        publisher: Arc<dyn ReviewPublisher>,
        scope: ReviewScope,
        diffs: FileDiffs,
        history: CommitHistory,
        progress: ProgressStore,
    ) -> Self {
        Self {
            store,
            publisher,
            scope,
            diffs,
            history,
            progress,
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

        let (pull_request, id, theirs, mine) = match self.publisher.readiness(&scope.branch)? {
            Readiness::Ready {
                pull_request,
                id,
                head,
                mine,
            } => (pull_request, id, head, mine),
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

        // Refused here rather than at the far end, where it comes back as a
        // bare "Unprocessable Entity" and the reviewer is left guessing which
        // of the things they just did was the problem.
        if mine && verdict != Verdict::Comment {
            return Err(CommentError::OwnPullRequest.into());
        }

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

        // After the review, and unable to bring it down. The review is posted
        // and cannot be taken back; the ticks are a convenience, so everything
        // that can go wrong from here is reported beside a review that went.
        // That includes working out which files to tick: a `?` on this line
        // would raise an error for a review that is already on the pull
        // request, which reads as though nothing had been sent.
        let read = self
            .already_read()
            .and_then(|paths| self.publisher.mark_read(&id, &paths));
        Ok(Sent {
            url,
            comments: waiting.len(),
            read: *read.as_ref().unwrap_or(&0),
            read_failed: read.err().map(|e| e.to_string()),
        })
    }

    /// The files the reviewer has read and that have not moved since.
    ///
    /// The same question the screen asks of every row: read is read *of this
    /// content*, so a file whose hash moved on is not read any more. Sending it
    /// as viewed would tick something on the pull request that nobody has read
    /// in the shape it is in now.
    ///
    /// Taken from the review window, which also means every path sent is one
    /// the pull request knows.
    fn already_read(&self) -> Result<Vec<String>> {
        self.progress.current_paths(&self.scope.get()?.files)
    }

    /// The ticks, and nothing else.
    ///
    /// Wanted on its own because a review is not always possible: on your own
    /// pull request GitHub takes a comment and nothing more, and a reader who
    /// has no comment to leave still wants the next round to show what changed.
    /// The same commit must be under review before its file marks can be sent.
    pub fn ticks_only(&self) -> Result<usize> {
        let scope = self.scope.get()?;
        if scope.dirty {
            return Err(CommentError::Uncommitted.into());
        }
        let (id, head) = match self.publisher.readiness(&scope.branch)? {
            Readiness::Ready { id, head, .. } => (id, head),
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
        if head != scope.head_sha {
            return Err(CommentError::HeadMoved {
                theirs: short(&head),
                ours: short(&scope.head_sha),
                behind: !self.history.is_ancestor(&head)?,
            }
            .into());
        }
        self.publisher.mark_read(&id, &self.already_read()?)
    }

    /// The same rule `Comments::add` applies, asked again at the last moment.
    ///
    /// Between writing a comment and sending it the reviewer may have pulled,
    /// and the diff is not the one the comment was written against any more.
    /// The ones that drifted are named together rather than one at a time: the
    /// reviewer has to go and look at each, and being sent back for the next
    /// one after every fix is the worse version of the same information.
    ///
    /// Asked of `reached` rather than worked out here, so that what publishing
    /// refuses and what the list shows cannot drift apart from each other.
    fn still_in_the_diff(&self, comments: &[Comment]) -> Result<()> {
        let drifted = reached(self.scope.get()?, &self.diffs, comments.to_vec())?.drifted;
        match drifted.is_empty() {
            true => Ok(()),
            false => Err(CommentError::NoLongerInDiff {
                comments: drifted.iter().map(Comment::at).collect(),
            }
            .into()),
        }
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
    use crate::diff::domain::{Hunk, Side};
    use crate::progress::application::MarkViewed;
    use crate::progress::domain::{Progress, ProgressRepository};
    use crate::testing::InMemoryProgressRepository;
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
    fn a_verdict_nobody_can_give_on_their_own_pull_request_is_refused_here() {
        // GitHub answers a bare "Unprocessable Entity" for this, which leaves
        // the reviewer guessing which of the things they just did was wrong.
        let publisher = Arc::new(FakePublisher::ready_at("head").mine());
        let publish = publish_with(publisher.clone(), &[]);

        let err = publish.execute(Verdict::Approve, "reads well").unwrap_err();

        assert!(err.to_string().contains("yours"), "{err}");
        assert!(publisher.nothing_sent());
    }

    #[test]
    fn a_comment_on_your_own_pull_request_goes_as_it_always_did() {
        // Only approving and asking for changes are refused. Commenting on
        // your own is ordinary, and it is the whole of what farol can offer
        // there.
        let publisher = Arc::new(FakePublisher::ready_at("head").mine());
        let publish = publish_with(publisher.clone(), &[]);

        publish.execute(Verdict::Comment, "worth a note").unwrap();

        assert_eq!(publisher.sent().verdict, Verdict::Comment);
    }

    #[test]
    fn the_ticks_can_go_without_a_review_at_all() {
        // What is left when a review is not possible: the reader still wants
        // the next round to show what changed.
        let publisher = Arc::new(FakePublisher::ready_at("head").mine());
        let source = Arc::new(FakeDiffSource::with_paths(&["src/a.rs"]));
        let (publish, progress, _) = with_progress(publisher.clone(), source);
        MarkViewed::new(progress)
            .execute("src/a.rs", "now")
            .unwrap();

        assert_eq!(publish.ticks_only().unwrap(), 1);
        assert_eq!(publisher.marked(), vec!["src/a.rs".to_string()]);
        assert!(publisher.nothing_sent(), "no review was asked for");
    }

    #[test]
    fn marks_only_refuses_a_different_pr_commit_without_sending_marks() {
        let publisher = Arc::new(FakePublisher::ready_at("another-head").mine());
        let publish = publish_with(publisher.clone(), &[]);
        assert!(
            publish
                .ticks_only()
                .unwrap_err()
                .to_string()
                .contains("pull request is at")
        );
        assert!(publisher.marked().is_empty());
        assert!(publisher.nothing_sent());
    }

    #[test]
    fn marks_only_refuses_uncommitted_work() {
        let publisher = Arc::new(FakePublisher::ready_at("head"));
        let source = Arc::new(FakeDiffSource::with_paths(&["src/a.rs"]).dirty());
        let publish = over(publisher.clone(), source);
        assert!(
            publish
                .ticks_only()
                .unwrap_err()
                .to_string()
                .contains("uncommitted")
        );
        assert!(publisher.marked().is_empty());
    }

    #[test]
    fn the_files_already_read_go_up_ticked_with_the_review() {
        // So a second round shows what changed instead of everything. The host
        // is told which files, and the reviewer is told how many went.
        let publisher = Arc::new(FakePublisher::ready_at("head"));
        let source = Arc::new(FakeDiffSource::with_paths(&["src/a.rs", "src/b.rs"]));
        let (publish, progress, _) = with_progress(publisher.clone(), source);
        MarkViewed::new(progress)
            .execute("src/a.rs", "now")
            .unwrap();

        let sent = publish.execute(Verdict::Approve, "").unwrap();

        assert_eq!(publisher.marked(), vec!["src/a.rs".to_string()]);
        assert_eq!(sent.read, 1);
        assert!(sent.read_failed.is_none());
    }

    #[test]
    fn a_file_that_changed_after_it_was_read_is_not_ticked() {
        // Read means read of this content: the screen reopens a file whose
        // hash moved on, and ticking it on the pull request would say somebody
        // read a version nobody has seen.
        let publisher = Arc::new(FakePublisher::ready_at("head"));
        let source = Arc::new(FakeDiffSource::with_paths(&["src/a.rs"]));
        let (publish, _, repo) = with_progress(publisher.clone(), source);
        // Written straight into the store, because the ordinary way of marking
        // a file pins it to the hash it has now, and this test is about the
        // one it had then.
        let mut stale = Progress::new();
        stale.mark("src/a.rs", "the hash it had back then", "then");
        repo.save(&stale).unwrap();

        publish.execute(Verdict::Approve, "").unwrap();

        assert!(publisher.marked().is_empty(), "{:?}", publisher.marked());
    }

    #[test]
    fn a_host_that_will_not_tick_does_not_undo_a_review_that_went() {
        // The review is on the pull request and cannot be taken back. The
        // ticks are a convenience, so their failure is reported beside it
        // rather than in place of it.
        let publisher =
            Arc::new(FakePublisher::ready_at("head").marking_fails("no permission for that"));
        let source = Arc::new(FakeDiffSource::with_paths(&["src/a.rs"]));
        let (publish, progress, _) = with_progress(publisher, source);
        MarkViewed::new(progress)
            .execute("src/a.rs", "now")
            .unwrap();

        let sent = publish.execute(Verdict::Approve, "").unwrap();

        assert_eq!(sent.url, "https://example.test/r1");
        assert_eq!(sent.read, 0);
        assert!(
            sent.read_failed.unwrap().contains("no permission"),
            "the reviewer has to be told which half did not happen"
        );
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
    fn a_comment_on_the_old_side_is_checked_against_the_old_side() {
        // The diff prints old lines 10 to 14, and the same numbers on the new
        // side. A comment on the old side of a line the pre-image never had is
        // as drifted as one on the new side, and one that is there must not be
        // refused for sitting where the new side does not reach.
        let (publisher, publish) = published();
        publish
            .store
            .save(&Comment {
                side: Side::Old,
                ..comment("1", 10, 12)
            })
            .unwrap();
        publish
            .store
            .save(&Comment {
                side: Side::Old,
                ..comment("2", 30, 30)
            })
            .unwrap();

        let err = publish
            .execute(Verdict::Comment, "Reads well.")
            .unwrap_err()
            .to_string();

        assert!(err.contains("src/a.rs:30 (old)"), "{err}");
        assert!(!err.contains("src/a.rs:10-12"), "{err}");
        assert!(publisher.nothing_sent());
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
        assert!(err.starts_with("Push first"), "{err}");
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

        assert!(err.starts_with("Pull first"), "{err}");
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
    #[test]
    fn a_store_that_cannot_say_what_was_read_does_not_undo_a_review_either() {
        // Working out which files to tick reads the progress store, and that
        // happens after the review is on the pull request. Raised as an error,
        // it would tell somebody nothing had been sent when everything was.
        let publisher = Arc::new(FakePublisher::ready_at("head"));
        let source = Arc::new(FakeDiffSource::with_paths(&["src/a.rs"]));
        let repo = Arc::new(InMemoryProgressRepository::broken());
        let publish = PublishReview::new(
            Arc::new(InMemoryComments::default()),
            publisher,
            ReviewScope::new(source.clone()),
            FileDiffs::new(source.clone()),
            CommitHistory::new(source.clone()),
            ProgressStore::new(repo, FileDiffs::new(source)),
        );

        let sent = publish.execute(Verdict::Approve, "").unwrap();

        assert_eq!(sent.url, "https://example.test/r1");
        assert_eq!(sent.read, 0);
        assert!(
            sent.read_failed.unwrap().contains("not readable"),
            "the half that did not happen has to be named"
        );
    }

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
        with_progress(publisher, source).0
    }

    /// The same, with what the reviewer has read handed back: the store, for a
    /// test that marks a file the ordinary way, and the repository under it,
    /// for one that has to seed a mark that has since gone stale.
    fn with_progress(
        publisher: Arc<FakePublisher>,
        source: Arc<FakeDiffSource>,
    ) -> (
        PublishReview,
        ProgressStore,
        Arc<InMemoryProgressRepository>,
    ) {
        let repo = Arc::new(InMemoryProgressRepository::default());
        let progress = ProgressStore::new(repo.clone(), FileDiffs::new(source.clone()));
        let publish = PublishReview::new(
            Arc::new(InMemoryComments::default()),
            publisher,
            ReviewScope::new(source.clone()),
            FileDiffs::new(source.clone()),
            CommitHistory::new(source),
            progress.clone(),
        );
        (publish, progress, repo)
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
            side: Side::New,
            from,
            to,
            body: "Why this order?".into(),
            published: None,
        }
    }
}
