//! The threads: writing, answering, closing, and handing one to the agent.

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;

use super::{away, fail, view};
use crate::cmd::ServerUseCases;
use crate::diff::domain::Side;

/// What the reviewer wrote back, which is everything still waiting for an
/// answer: closing a comment removes it, so there is nothing here to filter.
pub(super) async fn comments(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    match cases.comments.all() {
        Ok(found) => {
            Json(view::CommentsView::of(&found, cases.send_comment.harness())).into_response()
        }
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct NewComment {
    path: String,
    /// Which side the numbers are counted on. Absent is the new side, which is
    /// what every comment written before there was a choice was about.
    #[serde(default)]
    side: Side,
    from: u32,
    to: u32,
    body: String,
}

pub(super) async fn write_comment(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<NewComment>,
) -> impl IntoResponse {
    match cases
        .comments
        .add(&body.path, body.side, body.from, body.to, &body.body)
    {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CloseQuery {
    /// Set once the reviewer confirmed that the thread's pending comment goes
    /// too. Without it a thread with one is refused, as in the terminal.
    #[serde(default)]
    discard_pending: bool,
}

pub(super) async fn close_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Query(q): Query<CloseQuery>,
) -> impl IntoResponse {
    match away(move || cases.close_thread.execute(&id, q.discard_pending)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct NewReply {
    body: String,
}

/// The reviewer writing back under a comment. Sending it on is a second call,
/// the same one the Send button makes.
pub(super) async fn reply_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Json(reply): Json<NewReply>,
) -> impl IntoResponse {
    let author = crate::comments::domain::Author::Reviewer;
    match away(move || cases.comments.reply(&id, author, &reply.body)).await {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

/// Hand one comment to the agent session. Only the id travels: the text is
/// built on the server from the comment as stored.
pub(super) async fn send_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match away(move || cases.send_comment.execute(&id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}
