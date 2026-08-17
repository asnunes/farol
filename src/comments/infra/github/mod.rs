//! Speaking to GitHub over its REST API.
//!
//! The API and not the `gh` command line: the API has a version header and an
//! announced deprecation policy, while the output of a CLI is free to change
//! whenever its authors like — and a machine that reviews code is not obliged
//! to be a machine that has `gh` installed and logged in.
//!
//! What we ask and what we make of each answer is here; the asking itself is
//! in `http`. Nothing in either logs: the token is a secret that would
//! otherwise end up in a server log file that outlives it.

mod http;

use std::sync::Arc;

use serde::Serialize;

use crate::comments::domain::{CommentError, Credentials, Readiness, Review, ReviewPublisher};
use crate::error::Result;
use crate::shared::paths::Remote;
use http::{Http, encoded};

/// A pull request host, addressed by where the repository actually lives.
pub struct GitHub {
    remote: Remote,
    credentials: Arc<dyn Credentials>,
    http: Http,
}

impl GitHub {
    pub fn new(remote: Remote, credentials: Arc<dyn Credentials>) -> Self {
        let http = Http::new(&remote.host);
        Self {
            remote,
            credentials,
            http,
        }
    }
}

impl ReviewPublisher for GitHub {
    fn readiness(&self, branch: &str) -> Result<Readiness> {
        let Some(token) = self.credentials.token()? else {
            return Ok(Readiness::NoToken);
        };

        let owner = self.owner();
        let open = self.http.get(
            &token,
            &format!(
                "{}/pulls?state=open&head={}:{}",
                self.repo(),
                owner,
                encoded(branch)
            ),
        )?;
        if open.refused() {
            return Ok(Readiness::TokenRefused);
        }
        if let Some(pull) = open.json()?.and_then(first_pull) {
            return Ok(pull);
        }

        // No pull request, and two very different reasons for it. Asking the
        // host whether the branch is there beats reading `refs/remotes`, which
        // is only as fresh as the last fetch.
        let pushed = self.http.get(
            &token,
            &format!("{}/branches/{}", self.repo(), encoded(branch)),
        )?;
        if pushed.refused() {
            return Ok(Readiness::TokenRefused);
        }
        Ok(match pushed.ok() {
            false => Readiness::BranchNotPushed,
            true => Readiness::NoPullRequest {
                open_at: format!(
                    "https://{}/{}/compare/{}?expand=1",
                    self.remote.host,
                    self.remote.slug,
                    encoded(branch)
                ),
            },
        })
    }

    fn publish(&self, review: &Review) -> Result<String> {
        let Some(token) = self.credentials.token()? else {
            return Err(CommentError::NoToken.into());
        };

        let url = format!("{}/pulls/{}/reviews", self.repo(), review.pull_request);
        let body = serde_json::to_string(&Posted::from(review))?;
        let answer = self.http.post(&token, &url, body)?;

        if answer.refused() {
            return Err(CommentError::TokenRefused.into());
        }
        if !answer.ok() {
            return Err(CommentError::Refused {
                what: answer.complaint(),
            }
            .into());
        }
        answer
            .json()?
            .as_ref()
            .and_then(|body| body.get("html_url"))
            .and_then(|url| url.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                CommentError::Refused {
                    what: "the review was posted but the host did not say where".into(),
                }
                .into()
            })
    }
}

impl GitHub {
    /// The API root. GitHub Enterprise serves it under `/api/v3` on the same
    /// host as the pages; github.com serves it from a host of its own.
    fn api(&self) -> String {
        match self.remote.host.as_str() {
            "github.com" | "www.github.com" => "https://api.github.com".into(),
            other => format!("https://{other}/api/v3"),
        }
    }

    fn repo(&self) -> String {
        format!("{}/repos/{}", self.api(), self.remote.slug)
    }

    /// Whose fork the branch is on. The same repository the review is posted
    /// to, which is the only arrangement farol claims to handle.
    fn owner(&self) -> &str {
        self.remote
            .slug
            .split_once('/')
            .map(|(owner, _)| owner)
            .unwrap_or(&self.remote.slug)
    }
}

/// The first open pull request on the branch, and the commit it is showing.
fn first_pull(body: serde_json::Value) -> Option<Readiness> {
    let pull = body.as_array()?.first()?;
    Some(Readiness::Ready {
        pull_request: pull.get("number")?.as_u64()? as u32,
        head: pull.get("head")?.get("sha")?.as_str()?.to_string(),
    })
}

/// The review as the API takes it.
#[derive(Serialize)]
struct Posted<'a> {
    commit_id: &'a str,
    body: &'a str,
    event: &'static str,
    comments: Vec<AtLines<'a>>,
}

impl<'a> From<&'a Review> for Posted<'a> {
    fn from(review: &'a Review) -> Self {
        use crate::comments::domain::Verdict;
        Self {
            commit_id: &review.head,
            body: &review.summary,
            event: match review.verdict {
                Verdict::Comment => "COMMENT",
                Verdict::RequestChanges => "REQUEST_CHANGES",
                Verdict::Approve => "APPROVE",
            },
            comments: review.comments.iter().map(AtLines::from).collect(),
        }
    }
}

/// One comment, anchored. `side: RIGHT` is the file as it now reads, which is
/// the only side farol lets anyone comment on.
#[derive(Serialize)]
struct AtLines<'a> {
    path: &'a str,
    body: &'a str,
    line: u32,
    side: &'static str,
    /// Left out for a single line: sending `start_line` equal to `line` is a
    /// validation error rather than a one-line range.
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_side: Option<&'static str>,
}

impl<'a> From<&'a crate::comments::domain::Comment> for AtLines<'a> {
    fn from(comment: &'a crate::comments::domain::Comment) -> Self {
        let spans = comment.from != comment.to;
        Self {
            path: &comment.path,
            body: &comment.body,
            line: comment.to,
            side: "RIGHT",
            start_line: spans.then_some(comment.from),
            start_side: spans.then_some("RIGHT"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::{Comment, Verdict};

    fn review(comments: Vec<Comment>) -> Review {
        Review {
            pull_request: 12,
            head: "abc1234".into(),
            summary: "Reads well.".into(),
            verdict: Verdict::RequestChanges,
            comments,
        }
    }

    fn comment(from: u32, to: u32) -> Comment {
        Comment {
            id: "1".into(),
            path: "src/a.rs".into(),
            from,
            to,
            body: "Why this order?".into(),
            published: None,
        }
    }

    #[test]
    fn a_span_is_sent_as_a_range_and_a_single_line_is_not() {
        // `start_line` equal to `line` is a validation error at the far end,
        // not a range of one.
        let body =
            serde_json::to_value(Posted::from(&review(vec![comment(82, 116), comment(9, 9)])))
                .unwrap();

        let span = &body["comments"][0];
        assert_eq!(span["line"], 116);
        assert_eq!(span["start_line"], 82);
        assert_eq!(span["start_side"], "RIGHT");

        let one = &body["comments"][1];
        assert_eq!(one["line"], 9);
        assert!(one.get("start_line").is_none(), "{one}");
        assert!(one.get("start_side").is_none(), "{one}");
    }

    #[test]
    fn each_verdict_goes_out_as_the_event_the_api_names() {
        for (verdict, event) in [
            (Verdict::Comment, "COMMENT"),
            (Verdict::RequestChanges, "REQUEST_CHANGES"),
            (Verdict::Approve, "APPROVE"),
        ] {
            let body = serde_json::to_value(Posted::from(&Review {
                verdict,
                ..review(vec![])
            }))
            .unwrap();

            assert_eq!(body["event"], event);
            assert_eq!(body["commit_id"], "abc1234");
        }
    }

    #[test]
    fn the_api_root_follows_the_host_the_repository_is_on() {
        // A company's own GitHub serves the API from the same host it serves
        // the pages, and hardcoding api.github.com would send the token there.
        assert_eq!(github("github.com").api(), "https://api.github.com");
        assert_eq!(
            github("github.acme.example").api(),
            "https://github.acme.example/api/v3"
        );
    }

    #[test]
    fn the_repository_and_its_owner_come_off_the_remote() {
        let host = github("github.com");
        assert_eq!(host.repo(), "https://api.github.com/repos/asnunes/farol");
        assert_eq!(host.owner(), "asnunes");
    }

    #[test]
    fn the_open_pull_request_is_read_with_the_commit_it_is_showing() {
        // The commit is the whole reason to ask: comments anchor to line
        // numbers, and those only mean anything against one commit.
        let body = serde_json::json!([{"number": 12, "head": {"sha": "abc1234"}}]);

        assert_eq!(
            first_pull(body),
            Some(Readiness::Ready {
                pull_request: 12,
                head: "abc1234".into()
            })
        );
        assert_eq!(first_pull(serde_json::json!([])), None);
    }

    fn github(host: &str) -> GitHub {
        struct NoToken;
        impl Credentials for NoToken {
            fn token(&self) -> Result<Option<String>> {
                Ok(None)
            }
            fn set(&self, _: &str) -> Result<()> {
                Ok(())
            }
        }

        GitHub::new(
            Remote {
                host: host.into(),
                slug: "asnunes/farol".into(),
            },
            Arc::new(NoToken),
        )
    }
}
