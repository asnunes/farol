//! Speaking to GitHub over its REST API.
//!
//! The API and not the `gh` command line: the API has a version header and an
//! announced deprecation policy, while the output of a CLI is free to change
//! whenever its authors like — and a machine that reviews code is not obliged
//! to be a machine that has `gh` installed and logged in.
//!
//! Nothing here logs. The token is a secret that would otherwise end up in a
//! server log file that outlives it.

use std::sync::Arc;

use serde::Serialize;

use crate::comments::domain::{CommentError, Credentials, Readiness, Review, ReviewPublisher};
use crate::error::Result;
use crate::shared::paths::Remote;

/// A pull request host, addressed by where the repository actually lives.
pub struct GitHub {
    remote: Remote,
    credentials: Arc<dyn Credentials>,
    agent: ureq::Agent,
}

impl GitHub {
    pub fn new(remote: Remote, credentials: Arc<dyn Credentials>) -> Self {
        // A status is an answer, not a transport failure: a 404 means the
        // branch is not there, and that is something to read rather than to
        // fail on.
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .build();
        Self {
            remote,
            credentials,
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl ReviewPublisher for GitHub {
    fn readiness(&self, branch: &str) -> Result<Readiness> {
        let Some(token) = self.credentials.token()? else {
            return Ok(Readiness::NoToken);
        };

        let owner = self.owner();
        let open = self.get(
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
        let pushed = self.get(
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
        let answer = self.send(&token, &url, body)?;

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

    fn get(&self, token: &str, url: &str) -> Result<Answer> {
        let answer = self
            .agent
            .get(url)
            .header("authorization", &format!("Bearer {token}"))
            .header("accept", ACCEPT)
            .header("x-github-api-version", VERSION)
            .header("user-agent", AGENT)
            .call();
        Answer::of(answer, &self.remote.host)
    }

    fn send(&self, token: &str, url: &str, body: String) -> Result<Answer> {
        let answer = self
            .agent
            .post(url)
            .header("authorization", &format!("Bearer {token}"))
            .header("accept", ACCEPT)
            .header("x-github-api-version", VERSION)
            .header("user-agent", AGENT)
            .header("content-type", "application/json")
            .send(body);
        Answer::of(answer, &self.remote.host)
    }
}

/// The version of the API this was written against. Pinned on purpose: without
/// it, a change GitHub announces for new clients arrives here unannounced.
const VERSION: &str = "2022-11-28";
const ACCEPT: &str = "application/vnd.github+json";
const AGENT: &str = concat!("farol/", env!("CARGO_PKG_VERSION"));
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// What came back, with the status kept beside the body so that a refusal can
/// be read rather than thrown.
struct Answer {
    status: u16,
    body: String,
}

impl Answer {
    fn of(
        answer: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
        host: &str,
    ) -> Result<Self> {
        let mut answer = answer.map_err(|e| CommentError::Unreachable {
            host: host.to_string(),
            why: e.to_string(),
        })?;
        let status = answer.status().as_u16();
        let body = answer.body_mut().read_to_string().unwrap_or_default();
        Ok(Self { status, body })
    }

    fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 401 is a token the host would not take; 403 is one it took and will not
    /// let do this. Both are answered by going and fixing the token, so they
    /// read the same from here.
    fn refused(&self) -> bool {
        self.status == 401 || self.status == 403
    }

    fn json(&self) -> Result<Option<serde_json::Value>> {
        Ok(serde_json::from_str(&self.body).ok())
    }

    /// What the host said was wrong, in its own words where it gave any. The
    /// `errors` array is where the useful half lives — the top-level message
    /// for a rejected comment is only ever "Validation Failed".
    fn complaint(&self) -> String {
        let Ok(Some(body)) = self.json() else {
            return format!("HTTP {}", self.status);
        };
        let message = body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("no reason given");
        let details: Vec<&str> = body
            .get("errors")
            .and_then(|e| e.as_array())
            .map(|errors| {
                errors
                    .iter()
                    .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        match details.is_empty() {
            true => message.to_string(),
            false => format!("{message} — {}", details.join("; ")),
        }
    }
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

/// The first open pull request on the branch, and the commit it is showing.
fn first_pull(body: serde_json::Value) -> Option<Readiness> {
    let pull = body.as_array()?.first()?;
    Some(Readiness::Ready {
        pull_request: pull.get("number")?.as_u64()? as u32,
        head: pull.get("head")?.get("sha")?.as_str()?.to_string(),
    })
}

/// Branch names carry slashes, and a slash in a query value is a path.
fn encoded(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
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
    fn a_branch_name_with_a_slash_survives_being_put_in_a_url() {
        assert_eq!(encoded("feat/publish-review"), "feat%2Fpublish-review");
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

    #[test]
    fn a_rejection_is_reported_in_the_hosts_own_words() {
        // "Validation Failed" alone would leave the reviewer nothing to act on;
        // the array underneath is where the reason lives.
        let answer = Answer {
            status: 422,
            body: serde_json::json!({
                "message": "Validation Failed",
                "errors": [{"message": "line must be part of the diff"}],
            })
            .to_string(),
        };

        let said = answer.complaint();
        assert!(said.contains("Validation Failed"), "{said}");
        assert!(said.contains("part of the diff"), "{said}");
        assert!(!answer.ok());
        assert!(!answer.refused());
    }

    #[test]
    fn a_body_that_is_not_json_still_says_the_status() {
        let answer = Answer {
            status: 502,
            body: "<html>bad gateway</html>".into(),
        };

        assert_eq!(answer.complaint(), "HTTP 502");
    }

    #[test]
    fn both_ways_of_saying_no_to_a_token_read_the_same() {
        // 401 is a token the host will not take, 403 is one it took and will
        // not let do this. Both are fixed by going and fixing the token.
        for status in [401, 403] {
            let answer = Answer {
                status,
                body: String::new(),
            };
            assert!(answer.refused(), "{status}");
        }
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
