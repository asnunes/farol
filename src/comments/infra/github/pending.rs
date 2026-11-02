//! The reviewer's pending review, and submitting it.

use serde::Serialize;

use super::*;
use crate::comments::domain::{
    Lines, PendingComment, PendingReview, RemoteComment, RemoteState, Review,
};

impl PendingReview for GitHub {
    /// Into the pending review the reviewer already has, or into a new one.
    ///
    /// GitHub keeps one pending review per person per pull request, so this
    /// asks for it before starting one. A new one is anchored at `head`, which
    /// the use case has checked is the commit the reviewer is reading.
    fn add_pending(&self, pull: &str, head: &str, at: &Lines, body: &str) -> Result<String> {
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

    /// Read off the review threads rather than the pending review, because
    /// only a thread knows which side of the diff it is on, and a comment
    /// without its side could land on either.
    fn pending_comments(&self, pull: &str) -> Result<Vec<PendingComment>> {
        let token = self.token()?;
        let data =
            self.graphql_call(&token, PENDING_THREADS, serde_json::json!({ "pull": pull }))?;
        let threads = data
            .pointer("/node/reviewThreads/nodes")
            .and_then(|n| n.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(threads.iter().flat_map(pending_in).collect())
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
}

impl GitHub {
    /// The reviewer's pending review on a pull request, if they have one.
    /// GitHub shows a pending review to its author only, so the first one
    /// listed is theirs.
    fn pending_review(&self, token: &str, pull: &str) -> Result<Option<OpenReview>> {
        let data = self.graphql_call(token, PENDING_REVIEW, serde_json::json!({ "pull": pull }))?;
        Ok(data.pointer("/node/reviews/nodes/0").and_then(|review| {
            Some(OpenReview {
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
}

/// The reviewer's pending review, and whether anything is in it.
pub(super) struct OpenReview {
    id: String,
    comments: u64,
}

/// A new thread in a pending review, in GraphQL's variable names. The side is
/// sent at both ends of a span, because `line` is read against it and the
/// host's default is not something to lean on.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Thread {
    review: String,
    path: String,
    body: String,
    line: u32,
    side: &'static str,
    /// Null for a single line: a range of one is a validation error.
    start_line: Option<u32>,
    start_side: Option<&'static str>,
}

impl From<&Lines> for Thread {
    fn from(lines: &Lines) -> Self {
        let spans = lines.from != lines.to;
        let side = sided(lines.side);
        Self {
            review: String::new(),
            path: lines.path.clone(),
            body: String::new(),
            line: lines.to,
            side,
            start_line: spans.then_some(lines.from),
            start_side: spans.then_some(side),
        }
    }
}

/// The pending comments of one review thread, on the thread's lines. A thread
/// without a line is on the file as a whole, which farol has no place to draw.
pub(super) fn pending_in(thread: &serde_json::Value) -> Vec<PendingComment> {
    let side = |key: &str| unsided(thread.get(key).and_then(|s| s.as_str()));
    let (Some(path), Some(to)) = (
        thread.get("path").and_then(|p| p.as_str()),
        thread.get("line").and_then(|l| l.as_u64()),
    ) else {
        return vec![];
    };
    let from = thread
        .get("startLine")
        .and_then(|l| l.as_u64())
        .unwrap_or(to);
    let at = Lines {
        path: path.to_string(),
        side: side("diffSide"),
        from: from as u32,
        to: to as u32,
    };
    thread
        .pointer("/comments/nodes")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
        .filter(|c| c.get("state").and_then(|s| s.as_str()) == Some("PENDING"))
        .filter_map(|c| {
            Some(PendingComment {
                id: c.get("id")?.as_str()?.to_string(),
                body: c.get("body")?.as_str()?.to_string(),
                at: at.clone(),
            })
        })
        .collect()
}

/// One node of a `nodes(ids:)` answer, as farol reads it.
pub(super) fn remote_state(node: &serde_json::Value) -> RemoteState {
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

/// Where the review can now be read, off whatever the host answered with.
pub(super) fn where_it_landed(answer: serde_json::Value) -> Result<String> {
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

/// A verdict with nothing pending under it, posted in one call.
#[derive(Serialize)]
pub(super) struct Alone<'a> {
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

pub(super) fn event(review: &Review) -> &'static str {
    use crate::comments::domain::Verdict;
    match review.verdict {
        Verdict::Comment => "COMMENT",
        Verdict::RequestChanges => "REQUEST_CHANGES",
        Verdict::Approve => "APPROVE",
    }
}

/// farol's side in GitHub's vocabulary.
pub(super) fn sided(side: crate::diff::domain::Side) -> &'static str {
    use crate::diff::domain::Side;
    match side {
        Side::Old => "LEFT",
        Side::New => "RIGHT",
    }
}

/// GitHub's side in farol's vocabulary: the reverse of `sided`, beside it so
/// the two cannot drift apart. A thread with no side is on the new code.
fn unsided(side: Option<&str>) -> crate::diff::domain::Side {
    use crate::diff::domain::Side;
    match side {
        Some("LEFT") => Side::Old,
        _ => Side::New,
    }
}

pub(super) const PENDING_REVIEW: &str = "query($pull: ID!) { node(id: $pull) { ... on PullRequest { reviews(states: [PENDING], first: 1) { nodes { id comments { totalCount } } } } } }";
// ponytail: the first 100 threads and 50 comments each; paginate if a pull
// request ever carries more.
pub(super) const PENDING_THREADS: &str = "query($pull: ID!) { node(id: $pull) { ... on PullRequest { reviewThreads(first: 100) { nodes { path line startLine diffSide comments(first: 50) { nodes { id body state } } } } } } }";
pub(super) const NUMBER: &str =
    "query($pull: ID!) { node(id: $pull) { ... on PullRequest { number } } }";
pub(super) const START_REVIEW: &str = "mutation($pull: ID!, $head: GitObjectID!) { addPullRequestReview(input: {pullRequestId: $pull, commitOID: $head}) { pullRequestReview { id } } }";
pub(super) const ADD_THREAD: &str = "mutation($review: ID!, $path: String!, $body: String!, $line: Int!, $side: DiffSide!, $startLine: Int, $startSide: DiffSide) { addPullRequestReviewThread(input: {pullRequestReviewId: $review, path: $path, body: $body, line: $line, side: $side, startLine: $startLine, startSide: $startSide}) { thread { comments(first: 1) { nodes { id } } } } }";
pub(super) const EDIT_COMMENT: &str = "mutation($id: ID!, $body: String!) { updatePullRequestReviewComment(input: {pullRequestReviewCommentId: $id, body: $body}) { pullRequestReviewComment { id } } }";
pub(super) const DELETE_COMMENT: &str =
    "mutation($id: ID!) { deletePullRequestReviewComment(input: {id: $id}) { clientMutationId } }";
pub(super) const DELETE_REVIEW: &str = "mutation($id: ID!) { deletePullRequestReview(input: {pullRequestReviewId: $id}) { clientMutationId } }";
pub(super) const SUBMIT_REVIEW: &str = "mutation($review: ID!, $event: PullRequestReviewEvent!, $body: String) { submitPullRequestReview(input: {pullRequestReviewId: $review, event: $event, body: $body}) { pullRequestReview { url } } }";
pub(super) const NODES: &str = "query($ids: [ID!]!) { nodes(ids: $ids) { ... on PullRequestReviewComment { id body url state } } }";
