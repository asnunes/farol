//! The review itself: the map as the screen draws it, a file's diff, the lines
//! between hunks, and what has been read.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;

use super::{fail, view};
use crate::cmd::ServerUseCases;

/// Everything the screen needs to draw itself, in one call.
pub(super) async fn review(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    match cases.review.execute() {
        Ok((snapshot, lenses)) => Json(view::ReviewView::build(&snapshot, &lenses)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct PathQuery {
    path: String,
}

pub(super) async fn file(
    Extension(cases): Extension<ServerUseCases>,
    Query(q): Query<PathQuery>,
) -> impl IntoResponse {
    match cases.file_diff.execute(&q.path) {
        Ok(diff) => Json(view::FileDiffView::of(&diff)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct RangeQuery {
    path: String,
    from: u32,
    to: u32,
}

/// A stretch of the file the diff never printed, for the reader opening a gap.
///
/// Text and nothing else: which line each string is, and which line it was
/// before the change, the page works out from the hunks it already has.
pub(super) async fn lines(
    Extension(cases): Extension<ServerUseCases>,
    Query(q): Query<RangeQuery>,
) -> impl IntoResponse {
    match cases.file_lines.execute(&q.path, q.from, q.to) {
        Ok(lines) => Json(view::LinesView { lines }).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
pub(super) struct ViewedBody {
    path: String,
    viewed: bool,
}

pub(super) async fn viewed(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<ViewedBody>,
) -> impl IntoResponse {
    let result = if body.viewed {
        cases.mark_viewed.execute(&body.path, &now())
    } else {
        cases.unmark_viewed.execute(&body.path)
    };
    match result {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

pub(super) fn now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}
