use std::fmt::{self, Display};

use crate::comments::application::Standing;
use crate::comments::domain::Readiness;

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
