use std::fmt::{self, Display};

use crate::comments::application::Standing;
use crate::comments::domain::{ModeReason, Readiness, RemoteState, ReviewMode};

/// Where the review stands, for the terminal.
///
/// One line, opening with the state as a word, because the first reader of this
/// is a session deciding what to do next and the second is a person who wants
/// to know what is in the way. Both read the beginning of the line.
///
/// Named after what it prints rather than after the question it answers: this
/// is the third name the same idea already carries, after the domain's
/// `Readiness` and the application's `Standing`.
pub struct StandingLine<'a>(pub &'a Standing);

impl Display for StandingLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let branch = &self.0.branch;
        match &self.0.readiness {
            Readiness::Ready {
                pull_request,
                head,
                mine,
                ..
            } => writeln!(
                f,
                "ready: pull request #{pull_request}, showing {}\n{}",
                head,
                if *mine {
                    "This is your own pull request: publish comments or sync read marks only."
                } else {
                    "This pull request belongs to another author: choose a review verdict."
                }
            ),
            Readiness::Ambiguous { numbers } => writeln!(
                f,
                "ambiguous: more than one open pull request shows this commit: {}",
                numbers
                    .iter()
                    .map(|n| format!("#{n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Readiness::NoRemote => writeln!(
                f,
                "no remote: this repository has nowhere to send a review to"
            ),
            Readiness::NoToken => writeln!(
                f,
                "no token: GitHub CLI is not authenticated for this host\nRun `gh auth login --hostname {}`.",
                self.0.host.as_deref().unwrap_or("github.com")
            ),
            Readiness::TokenRefused => writeln!(
                f,
                "token refused: it expired, or it is missing Pull requests: Read and write or Contents: Read on this repository"
            ),
            Readiness::BranchNotPushed => {
                writeln!(f, "branch not pushed: '{branch}' is not on GitHub yet")
            }
            Readiness::NoPullRequest { open_at } => writeln!(
                f,
                "no pull request: '{branch}' is on GitHub with nothing open on it\n{open_at}"
            ),
        }?;
        if let Some(host) = &self.0.host {
            writeln!(f, "host: {host}")?;
        }
        let mode = match self.0.mode.mode {
            ReviewMode::SelfReview => "self review",
            ReviewMode::PeerReview => "peer review",
        };
        match &self.0.mode.why {
            ModeReason::Chosen => writeln!(f, "mode: {mode}, chosen")?,
            ModeReason::Author { login, .. } => {
                writeln!(f, "mode: {mode}, the pull request is by {login}")?
            }
            ModeReason::NoPullRequest => writeln!(f, "mode: {mode}, no pull request to tell")?,
        }
        // The pending comments, because the skill that publishes reads this to
        // show the reader what is about to go.
        let waiting: Vec<_> = self
            .0
            .pending
            .iter()
            .filter_map(|one| match &one.state {
                RemoteState::Pending { body } => Some((one, body)),
                _ => None,
            })
            .collect();
        writeln!(
            f,
            "{} pending comment(s) on the pull request:",
            waiting.len()
        )?;
        for (one, body) in waiting {
            writeln!(f, "  {} (thread {})", one.at, one.thread)?;
            for line in body.lines() {
                writeln!(f, "    {line}")?;
            }
        }
        if let Some(why) = &self.0.pending_failed {
            writeln!(f, "the pending comments could not be read: {why}")?;
        }
        writeln!(f, "{} current file(s) marked as read:", self.0.read.len())?;
        for path in &self.0.read {
            writeln!(f, "  {path}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(readiness: Readiness) -> Standing {
        Standing {
            host: Some("github.com".into()),
            branch: "feat/x".into(),
            readiness,
            mode: crate::comments::domain::ModeChoice::resolve(None, None),
            pending: vec![],
            pending_failed: None,
            read: vec![],
        }
    }

    #[test]
    fn status_tells_the_skill_when_the_pr_belongs_to_the_reader() {
        let mut standing = at(Readiness::Ready {
            pull_request: 12,
            id: "id".into(),
            mine: true,
            head: "head".into(),
            author: "asnunes".into(),
            url: "https://example.test/pull/12".into(),
        });
        standing.read = vec!["src/a.rs".into()];
        let text = StandingLine(&standing).to_string();
        assert!(text.contains("your own pull request"));
        assert!(text.contains("host: github.com"));
        assert!(text.contains("1 current file(s)"));
        assert!(text.contains("src/a.rs"));
    }

    #[test]
    fn every_state_opens_on_the_word_a_caller_branches_on() {
        // A session reads the first word and decides; a person reads the rest.
        // Neither should have to look past the start of the line to tell the
        // states apart.
        for (readiness, opening) in [
            (
                Readiness::Ready {
                    pull_request: 12,
                    id: "PR_kwDO".into(),
                    mine: false,
                    head: "abc1234def".into(),
                    author: "alice".into(),
                    url: "https://example.test/pull/12".into(),
                },
                "ready:",
            ),
            (Readiness::NoRemote, "no remote:"),
            (Readiness::NoToken, "no token:"),
            (Readiness::TokenRefused, "token refused:"),
            (Readiness::BranchNotPushed, "branch not pushed:"),
            (
                Readiness::NoPullRequest {
                    open_at: "https://example.test/compare".into(),
                },
                "no pull request:",
            ),
        ] {
            let said = StandingLine(&at(readiness)).to_string();
            assert!(said.starts_with(opening), "{said}");
        }
    }

    #[test]
    fn the_complete_commit_is_available_for_the_skill_to_verify() {
        // It is the sha a refused publish will compare against, so it belongs
        // in the answer, at the length every other tool prints.
        let said = StandingLine(&at(Readiness::Ready {
            pull_request: 12,
            id: "PR_kwDO".into(),
            mine: false,
            head: "abc1234def5678".into(),
            author: "alice".into(),
            url: "https://example.test/pull/12".into(),
        }))
        .to_string();

        assert!(said.contains("#12"), "{said}");
        assert!(said.contains("abc1234"), "{said}");
        assert!(said.contains("abc1234def5678"), "{said}");
    }

    #[test]
    fn the_branch_is_named_where_the_reader_has_to_act_on_it() {
        // 'push it' and 'open one for it' are both instructions about a branch,
        // and farol is often looking at a different one than the reader is.
        for readiness in [
            Readiness::BranchNotPushed,
            Readiness::NoPullRequest {
                open_at: "https://example.test/compare".into(),
            },
        ] {
            let said = StandingLine(&at(readiness)).to_string();
            assert!(said.contains("feat/x"), "{said}");
        }
    }

    #[test]
    fn the_page_that_opens_a_pull_request_travels_with_the_state_that_has_one() {
        let said = StandingLine(&at(Readiness::NoPullRequest {
            open_at: "https://example.test/compare".into(),
        }))
        .to_string();

        assert!(said.contains("https://example.test/compare"), "{said}");
    }
}
