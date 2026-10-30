//! What the browser asks about the pull request: its state, the review mode,
//! each thread's pending comment, and submitting the review.
//!
//! Apart from the rest of the routes because these are the ones that reach
//! GitHub. The router still names every path in one place, in the parent.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;

use super::{away, fail, view};
use crate::cmd::ServerUseCases;

/// The pull request, its mode and each thread's pending comment. Asked of
/// GitHub, so off the request thread.
pub(super) async fn pull_request(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    match away(move || cases.readiness.execute()).await {
        Ok(standing) => Json(view::PullRequestView::of(&standing)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct PendingBody {
    body: String,
}

/// The reviewer's own words for the pull request: written in the box, or
/// taken from a turn of the conversation. Either way it is the reviewer who
/// sends them, so they are signed by the reviewer.
pub(super) async fn write_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Json(pending): Json<PendingBody>,
) -> impl IntoResponse {
    let by = crate::comments::domain::Author::Reviewer;
    match away(move || cases.write_pending.execute(&id, &pending.body, by)).await {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

pub(super) async fn edit_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Json(pending): Json<PendingBody>,
) -> impl IntoResponse {
    match away(move || cases.edit_pending.execute(&id, &pending.body)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

pub(super) async fn remove_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match away(move || cases.remove_pending.execute(&id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize, Default)]
pub(super) struct GenerateBody {
    /// The reviewer's note for this one comment, from "Generate with
    /// instructions…". Their own words, going to their own agent, the same as
    /// a reply is.
    #[serde(default)]
    instruction: String,
}

pub(super) async fn generate_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    body: Option<Json<GenerateBody>>,
) -> impl IntoResponse {
    let instruction = body.map(|Json(b)| b.instruction).unwrap_or_default();
    match away(move || cases.generate_pending.execute(&id, &instruction)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum VerdictBody {
    Comment,
    RequestChanges,
    Approve,
}

#[derive(Deserialize)]
pub(super) struct SubmitBody {
    verdict: VerdictBody,
    #[serde(default)]
    summary: String,
}

/// Only the verdict and the summary travel: what is submitted is the pending
/// review already on the pull request, never text the request carries.
pub(super) async fn submit_review(
    Extension(cases): Extension<ServerUseCases>,
    Json(submit): Json<SubmitBody>,
) -> impl IntoResponse {
    use crate::comments::domain::Verdict;
    let verdict = match submit.verdict {
        VerdictBody::Comment => Verdict::Comment,
        VerdictBody::RequestChanges => Verdict::RequestChanges,
        VerdictBody::Approve => Verdict::Approve,
    };
    match away(move || cases.publish_review.execute(verdict, &submit.summary)).await {
        Ok(sent) => Json(view::SentView {
            url: sent.url,
            comments: sent.comments,
            read: sent.read,
            read_failed: sent.read_failed,
        })
        .into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct ModeBody {
    mode: crate::comments::domain::ReviewMode,
}

pub(super) async fn choose_mode(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<ModeBody>,
) -> impl IntoResponse {
    match away(move || cases.choose_mode.execute(body.mode)).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct PullBody {
    title: String,
    #[serde(default)]
    body: String,
}

/// Open the pull request, pushing the branch first when the host has not seen
/// it. GitHub's to answer, so off the request thread.
pub(super) async fn open_pull(
    Extension(cases): Extension<ServerUseCases>,
    Json(pull): Json<PullBody>,
) -> impl IntoResponse {
    let draft = crate::comments::domain::PullDraft {
        title: pull.title,
        body: pull.body,
    };
    match away(move || cases.open_pull_request.execute(draft)).await {
        Ok(opened) => Json(view::OpenedPullView {
            number: opened.number,
            url: opened.url,
        })
        .into_response(),
        Err(e) => fail(e),
    }
}

/// Ask the agent to write the pull request's title and description. It writes
/// them back through the CLI, and the page reloads the draft when it lands.
pub(super) async fn generate_pull_draft(
    Extension(cases): Extension<ServerUseCases>,
) -> impl IntoResponse {
    match away(move || cases.generate_pull_draft.execute()).await {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(e) => fail(e),
    }
}
