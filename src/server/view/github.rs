//! The pull request as the screen needs it, and what a submitted review came to.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::comments::application::Standing;
use crate::comments::domain::{Drift, Readiness, RemoteState, ReviewMode};

/// The pull request, as far as the screen needs it: whether it can be written
/// on, which mode the review is in and why, and each thread's pending comment
/// as GitHub now holds it.
///
/// Its own answer rather than part of the comments: it asks the network, and
/// the threads should not wait for that to be drawn.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct PullRequestView {
    pub connection: Connection,
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
        let mut view = Self {
            connection: Connection::NoRemote,
            number: None,
            mine: false,
            author: None,
            url: None,
            open_at: None,
            login: standing.login(),
            numbers: vec![],
            mode: ModeView {
                mode: standing.mode.mode,
                reason: standing.mode.why.to_string(),
            },
            pending: standing
                .pending
                .iter()
                .map(|one| {
                    let view = match &one.state {
                        RemoteState::Pending { body } => RemotePendingView {
                            state: PendingState::Pending,
                            body: Some(body.clone()),
                            url: None,
                        },
                        RemoteState::Published { body, url } => RemotePendingView {
                            state: PendingState::Published,
                            body: Some(body.clone()),
                            url: Some(url.clone()),
                        },
                        RemoteState::Gone => RemotePendingView {
                            state: PendingState::Gone,
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
                    path: one.at.path().to_string(),
                    lines: one.at.lines().map(|lines| SpanView {
                        side: lines.side,
                        from: lines.from,
                        to: lines.to,
                    }),
                    at: one.at.at(),
                    body: one.body.clone(),
                })
                .collect(),
            pending_failed: standing.pending_failed.clone(),
            drift: standing.drift.map(|drift| match drift {
                Drift::Uncommitted => DriftView {
                    kind: DriftKind::Uncommitted,
                    commits: None,
                },
                Drift::Unpushed { commits } => DriftView {
                    kind: DriftKind::Unpushed,
                    commits: Some(commits),
                },
                Drift::Newer => DriftView {
                    kind: DriftKind::Newer,
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
                view.connection = Connection::Ready;
                view.number = Some(*pull_request);
                view.mine = *mine;
                view.author = Some(author.clone());
                view.url = Some(url.clone());
            }
            Readiness::NoToken | Readiness::TokenRefused => view.connection = Connection::Connect,
            Readiness::BranchNotPushed => view.connection = Connection::NoPullRequest,
            Readiness::NoPullRequest { open_at } => {
                view.connection = Connection::NoPullRequest;
                view.open_at = Some(open_at.clone());
            }
            Readiness::Ambiguous { numbers } => {
                view.connection = Connection::Ambiguous;
                view.numbers = numbers.clone();
            }
            Readiness::NoRemote => {}
        }
        view
    }
}

/// Whether the pull request can be written on, in the five things the screen
/// says: fewer than the reasons, because everything a login fixes reads the
/// same to the reviewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub enum Connection {
    Ready,
    Connect,
    NoPullRequest,
    Ambiguous,
    NoRemote,
}

/// A pending comment as GitHub now holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub enum PendingState {
    Pending,
    Published,
    Gone,
}

/// How the commit under review stands against the pull request's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub enum DriftKind {
    Uncommitted,
    Unpushed,
    Newer,
}

/// The title and description the pull request will be opened with.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct PullDraftView {
    pub title: String,
    pub body: String,
}

/// The pull request the host just opened.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct OpenedPullView {
    pub number: u32,
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct ModeView {
    pub mode: ReviewMode,
    /// What the screen says beside it.
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct RemotePendingView {
    pub state: PendingState,
    pub body: Option<String>,
    pub url: Option<String>,
}

/// A pending comment with no thread, where it is.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct LoosePendingView {
    pub id: String,
    pub path: String,
    /// None for one on the file as a whole, or outdated by a later push: the
    /// screen draws it above the file's code rather than under a line.
    pub lines: Option<SpanView>,
    /// Where it is, the way a person reads it, with `(old)` on the old side:
    /// built once here rather than again by the screen, which once left the
    /// side out.
    pub at: String,
    pub body: String,
}

/// The lines a pending comment is on, read against `side`.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct SpanView {
    pub side: crate::diff::domain::Side,
    pub from: u32,
    pub to: u32,
}

/// The commit under review against the pull request's.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct DriftView {
    pub kind: DriftKind,
    /// How many commits the pull request is missing, when they are ours.
    pub commits: Option<u32>,
}

/// What a sync of the read marks came to.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct SyncedView {
    pub number: u32,
    pub marked: usize,
    pub unmarked: usize,
}

/// What a submitted review came to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct SentView {
    pub url: String,
    pub comments: usize,
    pub read: usize,
    pub read_failed: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::{Lines, ModeChoice, PendingAt, PendingComment};
    use crate::diff::domain::Side;

    #[test]
    fn a_pending_comment_with_no_thread_says_which_side_it_is_on() {
        // Line 12 removed and line 12 kept are two passages. The screen built
        // this itself once and dropped the side, so the comment read as being
        // about the new code.
        let standing = Standing {
            host: None,
            branch: "feat/x".into(),
            readiness: Readiness::NoRemote,
            mode: ModeChoice::resolve(None, None),
            pending: vec![],
            loose: vec![PendingComment {
                id: "PRRC_1".into(),
                body: "Why remove this?".into(),
                at: PendingAt::Lines(Lines {
                    path: "src/a.rs".into(),
                    side: Side::Old,
                    from: 12,
                    to: 12,
                }),
            }],
            pending_failed: None,
            drift: None,
            read: vec![],
            draft: None,
        };

        let view = PullRequestView::of(&standing);

        assert_eq!(view.loose[0].at, "src/a.rs:12 (old)");
    }
}
