use std::sync::Arc;

use crate::comments::application::PullRequestGate;
use crate::comments::domain::{
    BranchPusher, CommentError, OpenedPull, PullDraft, PullDraftStore, Readiness, ReviewPublisher,
};
use crate::diff::application::ReviewScope;
use crate::error::Result;

/// Open the pull request for the branch under review, into the review's base.
///
/// A branch the host has never seen is pushed first: the reviewer pressed one
/// button to have a pull request, and being told to go and push is the step
/// the button exists to save. Anything else in the way — no token, a pull
/// request already open — is refused with what to do.
#[derive(Clone)]
pub struct OpenPullRequest {
    gate: PullRequestGate,
    publisher: Arc<dyn ReviewPublisher>,
    pusher: Arc<dyn BranchPusher>,
    scope: ReviewScope,
    drafts: Arc<dyn PullDraftStore>,
}

impl OpenPullRequest {
    pub fn new(
        gate: PullRequestGate,
        publisher: Arc<dyn ReviewPublisher>,
        pusher: Arc<dyn BranchPusher>,
        scope: ReviewScope,
        drafts: Arc<dyn PullDraftStore>,
    ) -> Self {
        Self {
            gate,
            publisher,
            pusher,
            scope,
            drafts,
        }
    }

    pub fn execute(&self, draft: PullDraft) -> Result<OpenedPull> {
        let draft = PullDraft {
            title: draft.title.trim().to_string(),
            body: draft.body.trim().to_string(),
        };
        if draft.title.is_empty() {
            return Err(CommentError::NoTitle.into());
        }
        let scope = self.scope.get()?;
        let branch = scope.branch.clone();
        match self.gate.readiness()? {
            Readiness::NoPullRequest { .. } => {}
            Readiness::BranchNotPushed => self.pusher.push(&branch)?,
            Readiness::Ready { pull_request, .. } => {
                return Err(CommentError::AlreadyOpen {
                    branch,
                    number: pull_request,
                }
                .into());
            }
            Readiness::Ambiguous { numbers } => {
                return Err(CommentError::AmbiguousPullRequest { numbers }.into());
            }
            Readiness::NoRemote => return Err(CommentError::NoRemote.into()),
            Readiness::NoToken => return Err(CommentError::NoToken.into()),
            Readiness::TokenRefused => return Err(CommentError::TokenRefused.into()),
        }
        let opened = self
            .publisher
            .open_pull(base_branch(&scope.base_ref), &branch, &draft)?;
        self.drafts.clear()?;
        Ok(opened)
    }
}

/// The branch on the host the review compares against: `origin/main` is
/// `main` there.
fn base_branch(base_ref: &str) -> &str {
    let base = base_ref.strip_prefix("refs/remotes/").unwrap_or(base_ref);
    let base = base.strip_prefix("refs/heads/").unwrap_or(base);
    base.strip_prefix("origin/").unwrap_or(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::PullDraftStore;
    use crate::diff::application::CommitHistory;
    use crate::testing::{FakeDiffSource, FakePublisher, InMemoryPullDrafts, RecordingPusher};

    struct Opening {
        open: OpenPullRequest,
        publisher: Arc<FakePublisher>,
        pusher: Arc<RecordingPusher>,
        drafts: Arc<InMemoryPullDrafts>,
    }

    fn opening(readiness: Readiness) -> Opening {
        let source = Arc::new(FakeDiffSource::with_paths(&["a.rs"]));
        let scope = ReviewScope::new(source.clone());
        let publisher = Arc::new(FakePublisher::blocked(readiness));
        let pusher = Arc::new(RecordingPusher::default());
        let drafts = Arc::new(InMemoryPullDrafts::default());
        let gate =
            PullRequestGate::new(publisher.clone(), scope.clone(), CommitHistory::new(source));
        Opening {
            open: OpenPullRequest::new(
                gate,
                publisher.clone(),
                pusher.clone(),
                scope,
                drafts.clone(),
            ),
            publisher,
            pusher,
            drafts,
        }
    }

    fn titled(title: &str) -> PullDraft {
        PullDraft {
            title: title.into(),
            body: "Why.".into(),
        }
    }

    #[test]
    fn a_pushed_branch_gets_its_pull_request_into_the_review_base_and_the_draft_goes() {
        let o = opening(Readiness::NoPullRequest {
            open_at: "x".into(),
        });
        o.drafts.write(&titled("Old draft")).unwrap();

        let opened = o.open.execute(titled("Stop from the page")).unwrap();

        assert_eq!(opened.number, 13);
        assert_eq!(
            o.publisher.calls(),
            vec!["open feature/x into main: Stop from the page"]
        );
        assert!(
            o.pusher.pushed().is_empty(),
            "a branch already on the host is not pushed again"
        );
        assert_eq!(
            o.drafts.read().unwrap(),
            None,
            "once open, the draft has nothing left to say"
        );
    }

    #[test]
    fn a_branch_the_host_has_not_seen_is_pushed_first() {
        let o = opening(Readiness::BranchNotPushed);

        o.open.execute(titled("Stop from the page")).unwrap();

        assert_eq!(o.pusher.pushed(), vec!["feature/x"]);
        assert_eq!(o.publisher.calls().len(), 1);
    }

    #[test]
    fn nothing_is_opened_without_a_title_or_over_one_already_open() {
        let o = opening(Readiness::NoPullRequest {
            open_at: "x".into(),
        });
        assert!(
            o.open
                .execute(titled("  "))
                .unwrap_err()
                .to_string()
                .contains("needs a title")
        );

        let ready = opening(Readiness::Ready {
            pull_request: 12,
            id: "PR".into(),
            mine: true,
            head: "head".into(),
            author: "me".into(),
            url: "u".into(),
        });
        let refused = ready.open.execute(titled("Again")).unwrap_err().to_string();
        assert!(refused.contains("already open"), "{refused}");
        assert!(ready.publisher.calls().is_empty());
    }

    #[test]
    fn the_base_is_the_branch_on_the_host() {
        assert_eq!(base_branch("origin/main"), "main");
        assert_eq!(base_branch("main"), "main");
        assert_eq!(base_branch("refs/remotes/origin/develop"), "develop");
    }
}
