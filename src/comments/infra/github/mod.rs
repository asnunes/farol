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
use http::{Answer, Http, encoded};

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

    /// In two calls when there are comments to carry, and in one when there are
    /// not.
    ///
    /// Posting the verdict and the comments together makes the summary
    /// mandatory: the API asks for a body whenever the event is COMMENT or
    /// REQUEST_CHANGES. Left as a draft first and submitted after, which is the
    /// path GitHub's own pages take, the body is optional and the comments are
    /// allowed to be the whole of what the review says.
    ///
    /// With nothing to carry there is nothing to draft, and an empty draft is
    /// refused at submission anyway, so that case goes straight out.
    fn publish(&self, review: &Review) -> Result<String> {
        let Some(token) = self.credentials.token()? else {
            return Err(CommentError::NoToken.into());
        };

        let reviews = format!("{}/pulls/{}/reviews", self.repo(), review.pull_request);
        if review.comments.is_empty() {
            let alone = serde_json::to_string(&Alone::from(review))?;
            return where_it_landed(self.read(self.http.post(&token, &reviews, alone)?)?);
        }

        let drafted = serde_json::to_string(&Drafted::from(review))?;
        let draft = self.read(self.http.post(&token, &reviews, drafted)?)?;
        let Some(id) = draft.get("id").and_then(|id| id.as_u64()) else {
            return Err(CommentError::Refused {
                what: "the comments were drafted but the host did not name the draft".into(),
            }
            .into());
        };

        let submitted = serde_json::to_string(&Submitted::from(review))?;
        match self.read(
            self.http
                .post(&token, &format!("{reviews}/{id}/events"), submitted)?,
        ) {
            Ok(sent) => where_it_landed(sent),
            // The draft is already on the pull request and nobody submitted it,
            // so it would sit there waiting for a reviewer who thinks they sent
            // it. Taking it back costs one call and leaves the failure looking
            // like what it is: nothing happened.
            Err(e) => {
                let _ = self.http.delete(&token, &format!("{reviews}/{id}"));
                Err(e)
            }
        }
    }

    /// One request, however many files.
    ///
    /// The REST API has no word for this: whether a file is marked as read is
    /// per person, and only the GraphQL schema exposes it. So this is the one
    /// call that speaks the other language, and it aliases a mutation per path
    /// rather than sending a request each.
    ///
    /// The paths travel as variables and never inside the query text. A path
    /// with a quote in it would otherwise end the string and the rest would be
    /// read as GraphQL.
    fn mark_read(&self, pull: &str, paths: &[String]) -> Result<usize> {
        if paths.is_empty() {
            return Ok(0);
        }
        let Some(token) = self.credentials.token()? else {
            return Err(CommentError::NoToken.into());
        };

        let answer = self.read(self.http.post(
            &token,
            &self.graphql(),
            serde_json::to_string(&Mutations::over(pull, paths))?,
        )?)?;
        Ok(took(&answer))
    }
}

impl GitHub {
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
    fn graphql(&self) -> String {
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

/// The first open pull request on the branch, and the commit it is showing.
fn first_pull(body: serde_json::Value) -> Option<Readiness> {
    let pull = body.as_array()?.first()?;
    Some(Readiness::Ready {
        pull_request: pull.get("number")?.as_u64()? as u32,
        id: pull.get("node_id")?.as_str()?.to_string(),
        head: pull.get("head")?.get("sha")?.as_str()?.to_string(),
    })
}

/// Where the review can now be read, off whatever the host answered with.
fn where_it_landed(answer: serde_json::Value) -> Result<String> {
    answer
        .get("html_url")
        .and_then(|url| url.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            CommentError::Refused {
                what: "the review was posted but the host did not say where".into(),
            }
            .into()
        })
}

/// A mutation per path, aliased so one request carries them all.
///
/// Written out as text because a GraphQL document is text; what must never be
/// text is the data, which rides in `variables`.
#[derive(Serialize)]
struct Mutations {
    query: String,
    variables: serde_json::Value,
}

impl Mutations {
    fn over(pull: &str, paths: &[String]) -> Self {
        let args = (0..paths.len())
            .map(|i| format!("$p{i}: String!"))
            .collect::<Vec<_>>()
            .join(", ");
        let calls = (0..paths.len())
            .map(|i| {
                format!(
                    "m{i}: markFileAsViewed(input: {{pullRequestId: $id, path: $p{i}}}) \
                     {{ clientMutationId }}"
                )
            })
            .collect::<Vec<_>>()
            .join(" ");

        let mut variables = serde_json::Map::new();
        variables.insert("id".into(), pull.into());
        for (i, path) in paths.iter().enumerate() {
            variables.insert(format!("p{i}"), path.as_str().into());
        }

        Self {
            query: format!("mutation($id: ID!, {args}) {{ {calls} }}"),
            variables: serde_json::Value::Object(variables),
        }
    }
}

/// How many of the aliases came back with something in them.
///
/// GraphQL answers 200 and puts what failed in `errors`, so the status says
/// nothing here. Counting the aliases that are not null is also what makes a
/// path the pull request does not know fail on its own without taking the rest
/// of the request with it.
fn took(answer: &serde_json::Value) -> usize {
    answer
        .get("data")
        .and_then(|d| d.as_object())
        .map(|fields| fields.values().filter(|v| !v.is_null()).count())
        .unwrap_or(0)
}

/// The comments, as a review nobody has submitted yet. No event: that is what
/// leaves it pending, and pending is what makes the summary optional.
#[derive(Serialize)]
struct Drafted<'a> {
    commit_id: &'a str,
    comments: Vec<AtLines<'a>>,
}

impl<'a> From<&'a Review> for Drafted<'a> {
    fn from(review: &'a Review) -> Self {
        Self {
            commit_id: &review.head,
            comments: review.comments.iter().map(AtLines::from).collect(),
        }
    }
}

/// The verdict, sent at the draft once it is written.
#[derive(Serialize)]
struct Submitted<'a> {
    event: &'static str,
    body: &'a str,
}

impl<'a> From<&'a Review> for Submitted<'a> {
    fn from(review: &'a Review) -> Self {
        Self {
            event: event(review),
            body: &review.summary,
        }
    }
}

/// A verdict with no comments under it, which needs no draft to sit on.
#[derive(Serialize)]
struct Alone<'a> {
    commit_id: &'a str,
    event: &'static str,
    body: &'a str,
}

impl<'a> From<&'a Review> for Alone<'a> {
    fn from(review: &'a Review) -> Self {
        Self {
            commit_id: &review.head,
            event: event(review),
            body: &review.summary,
        }
    }
}

fn event(review: &Review) -> &'static str {
    use crate::comments::domain::Verdict;
    match review.verdict {
        Verdict::Comment => "COMMENT",
        Verdict::RequestChanges => "REQUEST_CHANGES",
        Verdict::Approve => "APPROVE",
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
        let body = serde_json::to_value(Drafted::from(&review(vec![
            comment(82, 116),
            comment(9, 9),
        ])))
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
            let with_comments = serde_json::to_value(Submitted::from(&Review {
                verdict,
                ..review(vec![comment(9, 9)])
            }))
            .unwrap();
            let alone = serde_json::to_value(Alone::from(&Review {
                verdict,
                ..review(vec![])
            }))
            .unwrap();

            assert_eq!(with_comments["event"], event);
            assert_eq!(alone["event"], event);
            assert_eq!(alone["commit_id"], "abc1234");
        }
    }

    #[test]
    fn the_draft_carries_no_verdict_and_no_summary() {
        // Those are what a draft is missing, and missing them is what keeps it
        // pending: submitted in one call, the API would demand a summary for
        // every verdict except approve.
        let body = serde_json::to_value(Drafted::from(&review(vec![comment(9, 9)]))).unwrap();

        assert!(body.get("event").is_none(), "{body}");
        assert!(body.get("body").is_none(), "{body}");
        assert_eq!(body["commit_id"], "abc1234");
    }

    #[test]
    fn a_review_with_nothing_under_it_goes_out_whole() {
        // An empty draft is refused at submission, so a verdict with no
        // comments takes the one-call road, where an empty summary is allowed.
        let body = serde_json::to_value(Alone::from(&Review {
            verdict: Verdict::Approve,
            summary: String::new(),
            ..review(vec![])
        }))
        .unwrap();

        assert_eq!(body["event"], "APPROVE");
        assert_eq!(body["body"], "");
        assert!(body.get("comments").is_none(), "{body}");
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
    fn one_request_carries_a_mutation_per_file() {
        // Thirty files ticked is one round trip, not thirty. The paths ride in
        // `variables`, so a path with a quote in it cannot end the query string
        // and have the rest read as GraphQL.
        let body = serde_json::to_value(Mutations::over(
            "PR_kwDO",
            &["src/a.rs".into(), "he said \"hi\".rs".into()],
        ))
        .unwrap();

        let query = body["query"].as_str().unwrap();
        assert!(query.contains("m0: markFileAsViewed"), "{query}");
        assert!(query.contains("m1: markFileAsViewed"), "{query}");
        assert!(
            query.contains("$id: ID!, $p0: String!, $p1: String!"),
            "{query}"
        );
        assert!(
            !query.contains("src/a.rs"),
            "the data belongs in the variables"
        );

        assert_eq!(body["variables"]["id"], "PR_kwDO");
        assert_eq!(body["variables"]["p0"], "src/a.rs");
        assert_eq!(body["variables"]["p1"], "he said \"hi\".rs");
    }

    #[test]
    fn a_path_the_pull_request_does_not_know_fails_on_its_own() {
        // GraphQL answers 200 and puts what went wrong in `errors`, so the
        // status says nothing. What counts is how many aliases came back with
        // something in them.
        let answered = serde_json::json!({
            "data": { "m0": { "clientMutationId": null }, "m1": null },
            "errors": [{ "message": "Could not resolve to a file", "path": ["m1"] }],
        });

        assert_eq!(took(&answered), 1);
    }

    #[test]
    fn the_other_language_is_served_from_its_own_place() {
        // Enterprise puts GraphQL beside the REST root rather than under it,
        // so the `/api/v3` of the other calls would be a 404 here.
        assert_eq!(
            github("github.com").graphql(),
            "https://api.github.com/graphql"
        );
        assert_eq!(
            github("github.acme.example").graphql(),
            "https://github.acme.example/api/graphql"
        );
    }

    #[test]
    fn the_open_pull_request_is_read_with_the_commit_it_is_showing() {
        // The commit is the whole reason to ask: comments anchor to line
        // numbers, and those only mean anything against one commit. The node id
        // comes along because marking a file as read needs it and there is no
        // second call that would hand it over.
        let body =
            serde_json::json!([{"number": 12, "node_id": "PR_kwDO", "head": {"sha": "abc1234"}}]);

        assert_eq!(
            first_pull(body),
            Some(Readiness::Ready {
                pull_request: 12,
                id: "PR_kwDO".into(),
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
