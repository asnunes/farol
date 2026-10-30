//! The threads: writing, answering, closing, and handing one to the agent.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;

use super::{away, fail, view};
use crate::cmd::ServerUseCases;
use crate::comments::domain::{Lines, Subject};
use crate::diff::domain::Side;
use crate::lenses::domain::{LensId, LensTarget};
use crate::map::domain::Slug;

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
    #[serde(flatten)]
    about: NewSubject,
    body: String,
}

/// What a new thread is about, as the page sends it: lines, a lens, or a
/// block's description. Untagged so a body written before there was a choice
/// still reads as lines.
#[derive(Deserialize)]
#[serde(untagged)]
enum NewSubject {
    Lines {
        path: String,
        /// Which side the numbers are counted on. Absent is the new side, which
        /// is what every comment written before there was a choice was about.
        #[serde(default)]
        side: Side,
        from: u32,
        to: u32,
    },
    Lens {
        lens: String,
        target: LensTarget,
    },
    Block {
        block: String,
    },
}

impl NewSubject {
    /// Raw ids turned into proven ones here, at the edge.
    fn parse(self) -> crate::error::Result<Subject> {
        Ok(match self {
            Self::Lines {
                path,
                side,
                from,
                to,
            } => Subject::Lines(Lines {
                path,
                side,
                from,
                to,
            }),
            Self::Lens { lens, target } => Subject::Lens {
                lens: LensId::parse(&lens)?,
                target,
            },
            Self::Block { block } => Subject::Block {
                slug: Slug::parse(&block)?,
            },
        })
    }
}

pub(super) async fn write_comment(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<NewComment>,
) -> impl IntoResponse {
    let written = body
        .about
        .parse()
        .and_then(|about| cases.comments.add_about(about, &body.body));
    match written {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

pub(super) async fn close_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match away(move || cases.close_thread.execute(&id)).await {
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
