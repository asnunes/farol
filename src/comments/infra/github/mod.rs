//! Speaking to GitHub over its REST API.
//!
//! Request validation and payloads stay in Farol; authentication comes from gh.

mod http;

use std::sync::Arc;

use serde::Serialize;

use crate::comments::domain::{
    Comment, CommentError, Credentials, Readiness, RemoteComment, RemoteState, Review,
    ReviewPublisher,
};
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
    fn readiness(&self, branch: &str, head: &str) -> Result<Readiness> {
        let Some(token) = self.credentials.token(&self.remote.host)? else {
            return Ok(Readiness::NoToken);
        };

        // By the commit first: it finds the pull request of a fork, and of a
        // branch checked out locally under a name of its own, where the branch
        // name would find nothing. A commit the host has never seen answers
        // 422, which is not a refusal — it is the next lookup's turn.
        let by_commit = self
            .http
            .get(&token, &format!("{}/commits/{}/pulls", self.repo(), head))?;
        if by_commit.refused() {
            return Ok(Readiness::TokenRefused);
        }
        let open: Vec<Pull> = match by_commit.ok() {
            true => pulls(by_commit.json()?)
                .into_iter()
                .filter(|pull| pull.open)
                .collect(),
            false => vec![],
        };
        let showing: Vec<&Pull> = open.iter().filter(|pull| pull.head == head).collect();
        let found = match (showing.as_slice(), open.as_slice()) {
            ([one], _) => Some((*one).clone()),
            // It holds this commit and has moved on since: the head check
            // downstream says which way to move.
            ([], [one]) => Some(one.clone()),
            ([], []) => None,
            _ => {
                return Ok(Readiness::Ambiguous {
                    numbers: open.iter().map(|pull| pull.number).collect(),
                });
            }
        };

        // Then by the branch, which is what still finds the pull request when
        // the local commits are not pushed yet.
        let found = match found {
            Some(pull) => Some(pull),
            None => {
                let by_branch = self.http.get(
                    &token,
                    &format!(
                        "{}/pulls?state=open&head={}:{}",
                        self.repo(),
                        self.owner(),
                        encoded(branch)
                    ),
                )?;
                if by_branch.refused() {
                    return Ok(Readiness::TokenRefused);
                }
                by_branch
                    .json()?
                    .and_then(|body| pulls(Some(body)).into_iter().next())
            }
        };

        if let Some(pull) = found {
            // One more call, and only here: the screen has to know whose pull
            // request this is before it offers a verdict, and the answer is
            // the same for every state that is not Ready.
            let me = self.read(self.http.get(&token, &format!("{}/user", self.api()))?)?;
            let me = me.get("login").and_then(|l| l.as_str()).unwrap_or_default();
            return Ok(Readiness::Ready {
                mine: !me.is_empty() && me == pull.author,
                pull_request: pull.number,
                id: pull.id,
                head: pull.head,
                author: pull.author,
                url: pull.url,
            });
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

    /// Into the pending review the reviewer already has, or into a new one.
    ///
    /// GitHub keeps one pending review per person per pull request, so this
    /// asks for it before starting one. A new one is anchored at `head`, which
    /// the use case has checked is the commit the reviewer is reading.
    fn add_pending(&self, pull: &str, head: &str, at: &Comment, body: &str) -> Result<String> {
        let token = self.token()?;
        let review = match self.pending_review(&token, pull)? {
            Some(review) => review.id,
            None => {
                let started = self.graphql_call(
                    &token,
                    START_REVIEW,
                    serde_json::json!({ "pull": pull, "head": head }),
                )?;
                string_at(
                    &started,
                    &["addPullRequestReview", "pullRequestReview", "id"],
                )
                .ok_or_else(|| refused("the host started no pending review"))?
            }
        };
        let mut thread = Thread::from(at);
        thread.review = review;
        thread.body = body.to_string();
        let added = self.graphql_call(&token, ADD_THREAD, serde_json::to_value(&thread)?)?;
        added
            .pointer("/addPullRequestReviewThread/thread/comments/nodes/0/id")
            .and_then(|id| id.as_str())
            .map(str::to_string)
            .ok_or_else(|| refused("the comment was not added to the pending review"))
    }

    fn edit_pending(&self, id: &str, body: &str) -> Result<()> {
        let token = self.token()?;
        self.graphql_call(
            &token,
            EDIT_COMMENT,
            serde_json::json!({ "id": id, "body": body }),
        )?;
        Ok(())
    }

    fn remove_pending(&self, id: &str) -> Result<()> {
        let token = self.token()?;
        let answer = self.graphql(&token, DELETE_COMMENT, serde_json::json!({ "id": id }))?;
        match (refusal(&answer), not_found(&answer)) {
            (Some(why), false) => Err(refused(&why)),
            _ => Ok(()),
        }
    }

    fn remote(&self, ids: &[String]) -> Result<Vec<RemoteComment>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let token = self.token()?;
        // Partial answers are the normal case here: a comment deleted on the
        // pull request's page comes back as a null node with an error beside
        // it, and the rest of the answer is still good.
        let answer = self.graphql(&token, NODES, serde_json::json!({ "ids": ids }))?;
        let nodes = answer
            .pointer("/data/nodes")
            .and_then(|n| n.as_array())
            .ok_or_else(|| refused(&refusal(&answer).unwrap_or_else(|| "no answer".into())))?;
        Ok(ids
            .iter()
            .map(|id| RemoteComment {
                id: id.clone(),
                state: nodes
                    .iter()
                    .find(|node| node.get("id").and_then(|i| i.as_str()) == Some(id))
                    .map(remote_state)
                    .unwrap_or(RemoteState::Gone),
            })
            .collect())
    }

    /// The pending review, submitted, or the verdict on its own.
    ///
    /// A pending review whose comments were all removed is deleted rather
    /// than submitted, because GitHub refuses an empty one — and it would
    /// otherwise sit there, invisible to everyone but the reviewer.
    fn submit(&self, review: &Review) -> Result<String> {
        let token = self.token()?;
        match self.pending_review(&token, &review.pull)? {
            Some(pending) if pending.comments > 0 => {
                let body = (!review.summary.is_empty()).then_some(review.summary.as_str());
                let sent = self.graphql_call(
                    &token,
                    SUBMIT_REVIEW,
                    serde_json::json!({ "review": pending.id, "event": event(review), "body": body }),
                )?;
                string_at(
                    &sent,
                    &["submitPullRequestReview", "pullRequestReview", "url"],
                )
                .ok_or_else(|| refused("the review was submitted but the host did not say where"))
            }
            leftover => {
                if let Some(empty) = leftover {
                    self.graphql_call(
                        &token,
                        DELETE_REVIEW,
                        serde_json::json!({ "id": empty.id }),
                    )?;
                }
                let pull_request = self.number(&token, &review.pull)?;
                let alone = serde_json::to_string(&Alone::from(review))?;
                where_it_landed(self.read(self.http.post(
                    &token,
                    &format!("{}/pulls/{pull_request}/reviews", self.repo()),
                    alone,
                )?)?)
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
        let token = self.token()?;
        let answer = self.read(self.http.post(
            &token,
            &self.graphql_url(),
            serde_json::to_string(&Mutations::over(pull, paths))?,
        )?)?;

        // Nothing marked and the host saying why is a refusal, not an empty
        // answer. Reported as a count it would arrive on the screen as "there
        // was nothing to tick", which is what somebody whose token lacks the
        // permission would be told.
        match (took(&answer), refusal(&answer)) {
            (0, Some(why)) => Err(CommentError::Refused { what: why }.into()),
            (marked, _) => Ok(marked),
        }
    }
}

impl GitHub {
    /// The reviewer's pending review on a pull request, if they have one.
    /// GitHub shows a pending review to its author only, so the first one
    /// listed is theirs.
    fn pending_review(&self, token: &str, pull: &str) -> Result<Option<PendingReview>> {
        let data = self.graphql_call(token, PENDING_REVIEW, serde_json::json!({ "pull": pull }))?;
        Ok(data.pointer("/node/reviews/nodes/0").and_then(|review| {
            Some(PendingReview {
                id: review.get("id")?.as_str()?.to_string(),
                comments: review.pointer("/comments/totalCount")?.as_u64()?,
            })
        }))
    }

    /// The number REST addresses a pull request by, off the id GraphQL does.
    fn number(&self, token: &str, pull: &str) -> Result<u64> {
        let data = self.graphql_call(token, NUMBER, serde_json::json!({ "pull": pull }))?;
        data.pointer("/node/number")
            .and_then(|n| n.as_u64())
            .ok_or_else(|| refused("the host does not know this pull request"))
    }

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

/// The pull requests a listing names, as farol needs them. Both listings —
/// by commit and by branch — answer in this shape.
fn pulls(body: Option<serde_json::Value>) -> Vec<Pull> {
    let Some(serde_json::Value::Array(all)) = body else {
        return vec![];
    };
    all.iter()
        .filter_map(|pull| {
            Some(Pull {
                number: pull.get("number")?.as_u64()? as u32,
                id: pull.get("node_id")?.as_str()?.to_string(),
                head: pull.get("head")?.get("sha")?.as_str()?.to_string(),
                author: pull.get("user")?.get("login")?.as_str()?.to_string(),
                url: pull
                    .get("html_url")
                    .and_then(|u| u.as_str())
                    .unwrap_or_default()
                    .to_string(),
                open: pull.get("state").and_then(|s| s.as_str()).unwrap_or("open") == "open",
            })
        })
        .collect()
}

/// What the listing says about one pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pull {
    number: u32,
    /// The host's own name for it, which GraphQL addresses it by.
    id: String,
    /// The commit it is showing, which every comment is anchored against.
    head: String,
    /// Who opened it, which decides whether approving is even on the table.
    author: String,
    /// Its page.
    url: String,
    /// The listing by commit includes merged and closed ones.
    open: bool,
}

/// The reviewer's pending review, and whether anything is in it.
struct PendingReview {
    id: String,
    comments: u64,
}

/// A new thread in a pending review, in GraphQL's variable names. The side is
/// sent at both ends of a span, because `line` is read against it and the
/// host's default is not something to lean on.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
    review: String,
    path: String,
    body: String,
    line: u32,
    side: &'static str,
    /// Null for a single line: a range of one is a validation error.
    start_line: Option<u32>,
    start_side: Option<&'static str>,
}

impl From<&Comment> for Thread {
    fn from(comment: &Comment) -> Self {
        let spans = comment.from != comment.to;
        let side = sided(comment.side);
        Self {
            review: String::new(),
            path: comment.path.clone(),
            body: String::new(),
            line: comment.to,
            side,
            start_line: spans.then_some(comment.from),
            start_side: spans.then_some(side),
        }
    }
}

/// One node of a `nodes(ids:)` answer, as farol reads it.
fn remote_state(node: &serde_json::Value) -> RemoteState {
    let body = node
        .get("body")
        .and_then(|b| b.as_str())
        .unwrap_or_default()
        .to_string();
    match node.get("state").and_then(|s| s.as_str()) {
        Some("PENDING") => RemoteState::Pending { body },
        Some(_) => RemoteState::Published {
            body,
            url: node
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string(),
        },
        None => RemoteState::Gone,
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

const PENDING_REVIEW: &str = "query($pull: ID!) { node(id: $pull) { ... on PullRequest { reviews(states: [PENDING], first: 1) { nodes { id comments { totalCount } } } } } }";
const NUMBER: &str = "query($pull: ID!) { node(id: $pull) { ... on PullRequest { number } } }";
const START_REVIEW: &str = "mutation($pull: ID!, $head: GitObjectID!) { addPullRequestReview(input: {pullRequestId: $pull, commitOID: $head}) { pullRequestReview { id } } }";
const ADD_THREAD: &str = "mutation($review: ID!, $path: String!, $body: String!, $line: Int!, $side: DiffSide!, $startLine: Int, $startSide: DiffSide) { addPullRequestReviewThread(input: {pullRequestReviewId: $review, path: $path, body: $body, line: $line, side: $side, startLine: $startLine, startSide: $startSide}) { thread { comments(first: 1) { nodes { id } } } } }";
const EDIT_COMMENT: &str = "mutation($id: ID!, $body: String!) { updatePullRequestReviewComment(input: {pullRequestReviewCommentId: $id, body: $body}) { pullRequestReviewComment { id } } }";
const DELETE_COMMENT: &str =
    "mutation($id: ID!) { deletePullRequestReviewComment(input: {id: $id}) { clientMutationId } }";
const DELETE_REVIEW: &str = "mutation($id: ID!) { deletePullRequestReview(input: {pullRequestReviewId: $id}) { clientMutationId } }";
const SUBMIT_REVIEW: &str = "mutation($review: ID!, $event: PullRequestReviewEvent!, $body: String) { submitPullRequestReview(input: {pullRequestReviewId: $review, event: $event, body: $body}) { pullRequestReview { url } } }";
const NODES: &str = "query($ids: [ID!]!) { nodes(ids: $ids) { ... on PullRequestReviewComment { id body url state } } }";

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

/// A verdict with nothing pending under it, posted in one call.
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

/// farol's side in GitHub's vocabulary.
fn sided(side: crate::diff::domain::Side) -> &'static str {
    use crate::diff::domain::Side;
    match side {
        Side::Old => "LEFT",
        Side::New => "RIGHT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::{Comment, Verdict};

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
            path: "src/a.rs".into(),
            side: crate::diff::domain::Side::New,
            from,
            to,
            body: "Why this order?".into(),
            pending: None,
            conversation: Default::default(),
        }
    }

    #[test]
    fn a_span_is_sent_as_a_range_and_a_single_line_is_not() {
        // A range of one is a validation error at the far end, not a range.
        let span = serde_json::to_value(Thread::from(&comment(82, 116))).unwrap();
        assert_eq!(span["line"], 116);
        assert_eq!(span["startLine"], 82);
        assert_eq!(span["startSide"], "RIGHT");

        let one = serde_json::to_value(Thread::from(&comment(9, 9))).unwrap();
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
        let old = Comment {
            side: crate::diff::domain::Side::Old,
            ..comment(82, 116)
        };
        let span = serde_json::to_value(Thread::from(&old)).unwrap();
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
    fn a_request_the_host_refused_outright_is_a_failure_and_not_a_zero() {
        // The shape a token without the permission comes back in: nothing
        // marked, and the reason only in `errors`. Counted, it would reach the
        // screen as "there was nothing to tick".
        let refused = serde_json::json!({
            "data": null,
            "errors": [{ "message": "Resource not accessible by personal access token" }],
        });

        assert_eq!(took(&refused), 0);
        assert_eq!(
            refusal(&refused).unwrap(),
            "Resource not accessible by personal access token"
        );

        assert!(refusal(&serde_json::json!({ "data": { "m0": {} } })).is_none());
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
    fn the_open_pull_request_is_read_with_the_commit_it_is_showing() {
        // The commit is the whole reason to ask: comments anchor to line
        // numbers, and those only mean anything against one commit. The node id
        // comes along because GraphQL addresses the pull request by it, and the
        // author because approving your own pull request is not a thing GitHub
        // allows.
        let body = serde_json::json!([
            {
                "number": 12,
                "node_id": "PR_kwDO",
                "head": {"sha": "abc1234"},
                "user": {"login": "asnunes"},
                "state": "open",
                "html_url": "https://github.com/asnunes/farol/pull/12",
            },
            {
                "number": 3,
                "node_id": "PR_old",
                "head": {"sha": "abc1234"},
                "user": {"login": "asnunes"},
                "state": "closed",
            },
        ]);

        let all = pulls(Some(body));
        assert_eq!(
            all[0],
            Pull {
                number: 12,
                id: "PR_kwDO".into(),
                head: "abc1234".into(),
                author: "asnunes".into(),
                url: "https://github.com/asnunes/farol/pull/12".into(),
                open: true,
            }
        );
        assert!(
            !all[1].open,
            "a merged or closed one is listed by commit too"
        );
        assert!(pulls(Some(serde_json::json!([]))).is_empty());
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
                .add_pending("pull", "head", &comment(1, 1), "hi")
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
        )
    }
}
