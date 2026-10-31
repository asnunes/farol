//! The pull request as the screen needs it, and what a submitted review came to.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::comments::application::Standing;
use crate::comments::domain::{Drift, ModeReason, Readiness, RemoteState, ReviewMode};

/// The pull request, as far as the screen needs it: whether it can be written
/// on, which mode the review is in and why, and each thread's pending comment
/// as GitHub now holds it.
///
/// Its own answer rather than part of the comments: it asks the network, and
/// the threads should not wait for that to be drawn.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestView {
    /// "ready", "connect", "no-pull-request", "ambiguous" or "no-remote" —
    /// the five things the screen says, which is fewer than the reasons:
    /// everything a login fixes reads the same to the reviewer.
    pub connection: &'static str,
    pub number: Option<u32>,
    pub mine: bool,
    pub author: Option<String>,
    /// The pull request's page, when there is one.
    pub url: Option<String>,
    /// The page that opens a pull request for the branch, when it is pushed.
    pub open_at: Option<String>,
    /// The command that connects, for the host the repository is on.
    pub login: String,
    pub numbers: Vec<u32>,
    pub mode: ModeView,
    /// By thread id.
    pub pending: BTreeMap<String, RemotePendingView>,
    /// Pending comments no thread points at: written on the pull request's
    /// page, or left behind by a thread that was closed.
    pub loose: Vec<LoosePendingView>,
    pub pending_failed: Option<String>,
    /// How the commit under review stands against the pull request's.
    pub drift: Option<DriftView>,
    /// What the pull request will say, while there is none open: written by
    /// the agent after Generate, for the form to start from.
    pub draft: Option<PullDraftView>,
}

impl PullRequestView {
    pub fn of(standing: &Standing) -> Self {
        let host = standing.host.as_deref().unwrap_or("github.com");
        let mut view = Self {
            connection: "no-remote",
            number: None,
            mine: false,
            author: None,
            url: None,
            open_at: None,
            login: format!("gh auth login --hostname {host}"),
            numbers: vec![],
            mode: ModeView {
                mode: match standing.mode.mode {
                    ReviewMode::SelfReview => "self",
                    ReviewMode::PeerReview => "peer",
                },
                reason: match &standing.mode.why {
                    ModeReason::Chosen => "chosen by you".into(),
                    ModeReason::Author { mine: true, .. } => "the pull request is yours".into(),
                    ModeReason::Author { login, .. } => format!("the pull request is by {login}"),
                    ModeReason::NoPullRequest => "no pull request to tell".into(),
                },
            },
            pending: standing
                .pending
                .iter()
                .map(|one| {
                    let view = match &one.state {
                        RemoteState::Pending { body } => RemotePendingView {
                            state: "pending",
                            body: Some(body.clone()),
                            url: None,
                        },
                        RemoteState::Published { body, url } => RemotePendingView {
                            state: "published",
                            body: Some(body.clone()),
                            url: Some(url.clone()),
                        },
                        RemoteState::Gone => RemotePendingView {
                            state: "gone",
                            body: None,
                            url: None,
                        },
                    };
                    (one.thread.clone(), view)
                })
                .collect(),
            loose: standing
                .loose
                .iter()
                .map(|one| LoosePendingView {
                    id: one.id.clone(),
                    path: one.at.path.clone(),
                    side: match one.at.side {
                        crate::diff::domain::Side::Old => "old",
                        crate::diff::domain::Side::New => "new",
                    },
                    from: one.at.from,
                    to: one.at.to,
                    body: one.body.clone(),
                })
                .collect(),
            pending_failed: standing.pending_failed.clone(),
            drift: standing.drift.map(|drift| match drift {
                Drift::Uncommitted => DriftView {
                    kind: "uncommitted",
                    commits: None,
                },
                Drift::Unpushed { commits } => DriftView {
                    kind: "unpushed",
                    commits: Some(commits),
                },
                Drift::Newer => DriftView {
                    kind: "newer",
                    commits: None,
                },
            }),
            draft: standing.draft.as_ref().map(|draft| PullDraftView {
                title: draft.title.clone(),
                body: draft.body.clone(),
            }),
        };
        match &standing.readiness {
            Readiness::Ready {
                pull_request,
                mine,
                author,
                url,
                ..
            } => {
                view.connection = "ready";
                view.number = Some(*pull_request);
                view.mine = *mine;
                view.author = Some(author.clone());
                view.url = Some(url.clone());
            }
            Readiness::NoToken | Readiness::TokenRefused => view.connection = "connect",
            Readiness::BranchNotPushed => view.connection = "no-pull-request",
            Readiness::NoPullRequest { open_at } => {
                view.connection = "no-pull-request";
                view.open_at = Some(open_at.clone());
            }
            Readiness::Ambiguous { numbers } => {
                view.connection = "ambiguous";
                view.numbers = numbers.clone();
            }
            Readiness::NoRemote => {}
        }
        view
    }
}

/// The title and description the pull request will be opened with.
#[derive(Debug, Serialize)]
pub struct PullDraftView {
    pub title: String,
    pub body: String,
}

/// The pull request the host just opened.
#[derive(Debug, Serialize)]
pub struct OpenedPullView {
    pub number: u32,
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeView {
    /// "self" or "peer".
    pub mode: &'static str,
    /// What the screen says beside it.
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePendingView {
    /// "pending", "published" or "gone".
    pub state: &'static str,
    pub body: Option<String>,
    pub url: Option<String>,
}

/// A pending comment with no thread, on the lines it is about.
#[derive(Debug, Serialize)]
pub struct LoosePendingView {
    pub id: String,
    pub path: String,
    /// "old" or "new".
    pub side: &'static str,
    pub from: u32,
    pub to: u32,
    pub body: String,
}

/// The commit under review against the pull request's.
#[derive(Debug, Serialize)]
pub struct DriftView {
    /// "uncommitted", "unpushed" or "newer".
    pub kind: &'static str,
    /// How many commits the pull request is missing, when they are ours.
    pub commits: Option<u32>,
}

/// What a sync of the read marks came to.
#[derive(Debug, Serialize)]
pub struct SyncedView {
    pub number: u32,
    pub marked: usize,
    pub unmarked: usize,
}

/// What a submitted review came to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SentView {
    pub url: String,
    pub comments: usize,
    pub read: usize,
    pub read_failed: Option<String>,
}
