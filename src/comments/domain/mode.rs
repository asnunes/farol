//! Whose review this is: the author reading their own branch, or somebody
//! reading another person's pull request.

use serde::{Deserialize, Serialize};

use crate::comments::domain::Readiness;
use crate::error::Result;

/// The mode in effect, and why — the screen says the reason, so a wrong guess
/// is one click from being corrected rather than a mystery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeChoice {
    pub mode: ReviewMode,
    pub why: ModeReason,
}

impl ModeChoice {
    /// A choice somebody made wins. Then the pull request, when farol can see
    /// one. Then what the pull request said last time, because its author does
    /// not change and the agent's rules should not flip with the network. With
    /// none of those, self review: peer review without a pull request has
    /// nowhere to keep a pending comment.
    pub fn resolve(stored: Option<&StoredMode>, readiness: Option<&Readiness>) -> Self {
        if let Some(stored) = stored.filter(|stored| stored.chosen) {
            return Self {
                mode: stored.mode,
                why: ModeReason::Chosen,
            };
        }
        if let Some(Readiness::Ready { mine, author, .. }) = readiness {
            return Self::by(author, *mine);
        }
        match stored.and_then(|stored| stored.author.as_ref().map(|a| (a, stored.mode))) {
            Some((author, mode)) => Self::by(author, mode == ReviewMode::SelfReview),
            None => Self {
                mode: ReviewMode::SelfReview,
                why: ModeReason::NoPullRequest,
            },
        }
    }

    /// What to write down so the next resolution does not need the network:
    /// a choice, or the author the pull request named. Nothing when there was
    /// nothing to learn, so a pull request opened later is still looked at.
    pub fn to_store(&self) -> Option<StoredMode> {
        match &self.why {
            ModeReason::Chosen => Some(StoredMode {
                mode: self.mode,
                chosen: true,
                author: None,
            }),
            ModeReason::Author { login, .. } => Some(StoredMode {
                mode: self.mode,
                chosen: false,
                author: Some(login.clone()),
            }),
            ModeReason::NoPullRequest => None,
        }
    }

    fn by(author: &str, mine: bool) -> Self {
        Self {
            mode: match mine {
                true => ReviewMode::SelfReview,
                false => ReviewMode::PeerReview,
            },
            why: ModeReason::Author {
                login: author.to_string(),
                mine,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewMode {
    /// The branch is the reviewer's own; the agent is the one that wrote it.
    #[serde(rename = "self")]
    SelfReview,
    /// Somebody else's pull request; the agent is an assistant to the reader.
    #[serde(rename = "peer")]
    PeerReview,
}

impl ReviewMode {
    /// The rule every message to the agent carries, which no instruction in
    /// Settings can take out.
    ///
    /// Peer review says what not to do and nothing about whose pull request it
    /// is: a forced mode may be wrong about that, and the rule is what matters.
    pub fn rule(&self) -> &'static str {
        match self {
            ReviewMode::SelfReview => {
                "This is a self review of your branch. You may change the code to address the thread."
            }
            ReviewMode::PeerReview => {
                "You are in peer review mode: do not change, commit or push code in this repository."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeReason {
    /// Set with `--self-review`, `--peer-review` or the switch on screen.
    Chosen,
    /// Read off the pull request.
    Author { login: String, mine: bool },
    /// Nothing to read it off.
    NoPullRequest,
}

/// What `review-mode.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMode {
    pub mode: ReviewMode,
    pub chosen: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// Where the mode is remembered for the branch. A file of its own, because the
/// CLI and the server both write it and `state.json` belongs to progress.
pub trait ModeStore: Send + Sync {
    fn read(&self) -> Result<Option<StoredMode>>;
    fn write(&self, mode: &StoredMode) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(mine: bool) -> Readiness {
        Readiness::Ready {
            pull_request: 7,
            id: "PR_1".into(),
            head: "abc".into(),
            mine,
            author: "alice".into(),
            url: "https://example.test/pull/7".into(),
        }
    }

    #[test]
    fn a_pull_request_by_somebody_else_is_peer_review() {
        let choice = ModeChoice::resolve(None, Some(&ready(false)));
        assert_eq!(choice.mode, ReviewMode::PeerReview);
        assert_eq!(
            choice.to_store().unwrap().author.as_deref(),
            Some("alice"),
            "the author is remembered so the next send needs no network"
        );
    }

    #[test]
    fn a_choice_wins_over_the_pull_request() {
        let chosen = StoredMode {
            mode: ReviewMode::PeerReview,
            chosen: true,
            author: None,
        };
        let choice = ModeChoice::resolve(Some(&chosen), Some(&ready(true)));
        assert_eq!(choice.mode, ReviewMode::PeerReview);
        assert_eq!(choice.why, ModeReason::Chosen);
    }

    #[test]
    fn without_a_pull_request_it_is_what_the_pull_request_said_last_time() {
        let remembered = ModeChoice::resolve(None, Some(&ready(false))).to_store();
        let offline = ModeChoice::resolve(remembered.as_ref(), None);
        assert_eq!(offline.mode, ReviewMode::PeerReview);

        let never = ModeChoice::resolve(None, Some(&Readiness::NoToken));
        assert_eq!(never.mode, ReviewMode::SelfReview);
        assert_eq!(never.to_store(), None, "nothing learned, nothing written");
    }
}
