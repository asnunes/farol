//! Speaking to GitHub over its REST API.
//!
//! Request validation and payloads stay in Farol; authentication comes from gh.

mod http;
mod logins;
mod marks;
mod pending;
mod pull;

pub use logins::Logins;

use std::sync::Arc;

use crate::comments::domain::{CommentError, Credentials};
use crate::error::Result;
use crate::shared::paths::Remote;
use http::{Answer, Http, encoded};

/// A pull request host, addressed by where the repository actually lives.
pub struct GitHub {
    remote: Remote,
    credentials: Arc<dyn Credentials>,
    http: Http,
    logins: Arc<Logins>,
}

impl GitHub {
    pub fn new(remote: Remote, credentials: Arc<dyn Credentials>, logins: Arc<Logins>) -> Self {
        let http = Http::new(&remote.host);
        Self {
            remote,
            credentials,
            http,
            logins,
        }
    }
}

impl GitHub {
    /// A GraphQL request, answered whole: `data` and `errors` side by side,
    /// for the callers to which a partial answer is still an answer.
    fn graphql(
        &self,
        token: &str,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        self.read(
            self.http
                .post(token, &self.graphql_url(), serde_json::to_string(&body)?)?,
        )
    }

    /// The same, for the callers to which any error is a refusal. GraphQL
    /// answers 200 either way, so the status says nothing here.
    fn graphql_call(
        &self,
        token: &str,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let answer = self.graphql(token, query, variables)?;
        if let Some(why) = refusal(&answer) {
            return Err(refused(&why));
        }
        Ok(answer.get("data").cloned().unwrap_or_default())
    }

    /// The token, or the refusal that says there is none.
    ///
    /// `readiness` does not use this: without a token it has a state to
    /// report rather than an error to raise, which is the whole point of that
    /// question.
    fn token(&self) -> Result<String> {
        self.credentials
            .token(&self.remote.host)?
            .ok_or_else(|| CommentError::NoToken.into())
    }

    /// What the host said, as JSON, or the refusal in farol's own words.
    fn read(&self, answer: Answer) -> Result<serde_json::Value> {
        if answer.refused() {
            return Err(CommentError::TokenRefused.into());
        }
        if !answer.ok() {
            return Err(CommentError::Refused {
                what: answer.complaint(),
            }
            .into());
        }
        Ok(answer.json()?.unwrap_or(serde_json::Value::Null))
    }

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

    /// Where the other language is served. Enterprise puts it beside the REST
    /// root rather than under it: `/api/graphql`, not `/api/v3/graphql`.
    fn graphql_url(&self) -> String {
        match self.remote.host.as_str() {
            "github.com" | "www.github.com" => "https://api.github.com/graphql".into(),
            other => format!("https://{other}/api/graphql"),
        }
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

fn string_at(data: &serde_json::Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(data, |at, key| at.get(key))?
        .as_str()
        .map(str::to_string)
}

fn refused(what: &str) -> crate::error::Error {
    CommentError::Refused {
        what: what.to_string(),
    }
    .into()
}

/// Whether the host's only complaint is that the thing is not there.
fn not_found(answer: &serde_json::Value) -> bool {
    answer
        .get("errors")
        .and_then(|e| e.as_array())
        .is_some_and(|errors| {
            !errors.is_empty()
                && errors
                    .iter()
                    .all(|e| e.get("type").and_then(|t| t.as_str()) == Some("NOT_FOUND"))
        })
}

/// What the host said went wrong, when it said anything.
///
/// GraphQL answers 200 and puts the failures here, so this is the only place a
/// refusal is written down: a missing permission arrives as a message beside a
/// `data` full of nulls.
fn refusal(answer: &serde_json::Value) -> Option<String> {
    let said: Vec<&str> = answer
        .get("errors")?
        .as_array()?
        .iter()
        .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
        .collect();
    match said.is_empty() {
        true => None,
        false => Some(said.join("; ")),
    }
}

#[cfg(test)]
mod tests {
    use super::pending::*;
    use super::*;
    use crate::comments::domain::{Comment, Verdict};
    use crate::comments::domain::{Lines, Subject};
    use crate::comments::domain::{
        PendingReview, PullRequests, ReadMarks, Readiness, RemoteState, Review,
    };

    fn review() -> Review {
        Review {
            pull: "PR_kwDO".into(),
            head: "abc1234".into(),
            summary: "Reads well.".into(),
            verdict: Verdict::RequestChanges,
        }
    }

    fn comment(from: u32, to: u32) -> Comment {
        Comment {
            id: "1".into(),
            about: Subject::Lines(Lines {
                path: "src/a.rs".into(),
                side: crate::diff::domain::Side::New,
                from,
                to,
            }),
            body: "Why this order?".into(),
            pending: None,
            conversation: Default::default(),
        }
    }

    #[test]
    fn only_the_pending_comments_of_a_thread_come_back_on_its_lines() {
        let thread = serde_json::json!({
            "path": "src/a.rs", "line": 12, "startLine": 10, "diffSide": "LEFT",
            "comments": { "nodes": [
                { "id": "PRRC_1", "body": "Already out.", "state": "SUBMITTED" },
                { "id": "PRRC_2", "body": "Still mine.", "state": "PENDING" },
            ] },
        });

        let found = pending_in(&thread);

        assert_eq!(
            found.len(),
            1,
            "a submitted reply is the pull request's, not pending"
        );
        assert_eq!(found[0].id, "PRRC_2");
        assert_eq!(
            (found[0].at.from, found[0].at.to, found[0].at.side),
            (10, 12, crate::diff::domain::Side::Old)
        );
    }

    #[test]
    fn a_span_is_sent_as_a_range_and_a_single_line_is_not() {
        // A range of one is a validation error at the far end, not a range.
        let span =
            serde_json::to_value(Thread::from(comment(82, 116).about.lines().unwrap())).unwrap();
        assert_eq!(span["line"], 116);
        assert_eq!(span["startLine"], 82);
        assert_eq!(span["startSide"], "RIGHT");

        let one = serde_json::to_value(Thread::from(comment(9, 9).about.lines().unwrap())).unwrap();
        assert_eq!(one["line"], 9);
        assert_eq!(one["side"], "RIGHT");
        assert!(one["startLine"].is_null(), "{one}");
        assert!(one["startSide"].is_null(), "{one}");
    }

    #[test]
    fn a_comment_on_the_old_side_goes_out_as_left_at_both_ends() {
        // The numbers are read against the side, so a span sent without
        // `startSide` would have its first line taken on the right and land
        // somewhere nobody wrote about.
        let old = crate::comments::domain::Comment::on_side(
            comment(82, 116),
            crate::diff::domain::Side::Old,
        );
        let span = serde_json::to_value(Thread::from(old.about.lines().unwrap())).unwrap();
        assert_eq!(span["side"], "LEFT");
        assert_eq!(span["startSide"], "LEFT");
        assert_eq!(span["startLine"], 82);
    }

    #[test]
    fn each_verdict_goes_out_as_the_event_the_api_names() {
        for (verdict, name) in [
            (Verdict::Comment, "COMMENT"),
            (Verdict::RequestChanges, "REQUEST_CHANGES"),
            (Verdict::Approve, "APPROVE"),
        ] {
            let alone = serde_json::to_value(Alone::from(&Review {
                verdict,
                ..review()
            }))
            .unwrap();
            assert_eq!(alone["event"], name);
            assert_eq!(alone["commit_id"], "abc1234");
            assert_eq!(
                event(&Review {
                    verdict,
                    ..review()
                }),
                name
            );
        }
    }

    #[test]
    fn a_review_with_nothing_under_it_goes_out_whole() {
        let body = serde_json::to_value(Alone::from(&Review {
            verdict: Verdict::Approve,
            summary: String::new(),
            ..review()
        }))
        .unwrap();

        assert_eq!(body["event"], "APPROVE");
        assert_eq!(body["body"], "");
        assert!(body.get("comments").is_none(), "{body}");
    }

    #[test]
    fn what_the_host_holds_is_read_as_pending_published_or_gone() {
        assert_eq!(
            remote_state(&serde_json::json!({"id": "a", "body": "Hi", "state": "PENDING"})),
            RemoteState::Pending { body: "Hi".into() }
        );
        assert_eq!(
            remote_state(&serde_json::json!({
                "id": "a", "body": "Hi", "state": "SUBMITTED", "url": "https://x/1"
            })),
            RemoteState::Published {
                body: "Hi".into(),
                url: "https://x/1".into()
            }
        );
        assert_eq!(remote_state(&serde_json::json!({})), RemoteState::Gone);
    }

    #[test]
    fn a_comment_already_gone_is_not_a_failure_to_remove_it() {
        // Deleted on the pull request's page first: removing it from here has
        // nothing left to do, and saying it failed would leave the thread stuck.
        let gone = serde_json::json!({
            "data": null,
            "errors": [{ "type": "NOT_FOUND", "message": "Could not resolve to a node" }],
        });
        assert!(not_found(&gone));
        let forbidden = serde_json::json!({
            "errors": [{ "type": "FORBIDDEN", "message": "no" }],
        });
        assert!(!not_found(&forbidden));
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
    fn the_other_language_is_served_from_its_own_place() {
        // Enterprise puts GraphQL beside the REST root rather than under it,
        // so the `/api/v3` of the other calls would be a 404 here.
        assert_eq!(
            github("github.com").graphql_url(),
            "https://api.github.com/graphql"
        );
        assert_eq!(
            github("github.acme.example").graphql_url(),
            "https://github.acme.example/api/graphql"
        );
    }

    #[test]
    fn an_unauthorized_host_receives_no_requests_from_any_publishing_action() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let credentials = Arc::new(crate::testing::FakeCredentials::default());
        credentials.set("github.com", "test-secret").unwrap();
        let github = GitHub::new(
            Remote {
                host: listener.local_addr().unwrap().to_string(),
                slug: "owner/repo".into(),
            },
            credentials,
            Arc::default(),
        );

        assert_eq!(
            github.readiness("feature/x", "head").unwrap(),
            Readiness::NoToken
        );
        let error = github.submit(&review()).unwrap_err();
        assert!(error.to_string().contains("no token"), "{error}");
        assert!(github.mark_read("pull", &["a.rs".into()]).is_err());
        assert!(
            github
                .add_pending("pull", "head", comment(1, 1).about.lines().unwrap(), "hi")
                .is_err()
        );
        assert!(github.edit_pending("id", "hi").is_err());
        assert!(github.remove_pending("id").is_err());
        assert!(github.remote(&["id".into()]).is_err());
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "the host must not receive even a connection when its credential is absent"
        );
    }

    fn github(host: &str) -> GitHub {
        GitHub::new(
            Remote {
                host: host.into(),
                slug: "asnunes/farol".into(),
            },
            Arc::new(crate::testing::FakeCredentials::default()),
            Arc::default(),
        )
    }
}
