//! The four things the browser asks for.
//!
//! Handlers only: each one unpacks the request, calls one use case, and turns
//! the answer into a response. No business logic passes through here — that is
//! the point of the layer.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use tokio::sync::broadcast;

use super::registry::ServerEntry;
use super::{AppState, assets, view};
use crate::error::Error;

/// The whole HTTP surface. Everything it serves arrives injected, so the routes
/// can be exercised over fakes with nothing listening on a port.
pub(super) fn router(state: Arc<AppState>, identity: ServerEntry) -> Router {
    Router::new()
        .route("/api/review", get(review))
        .route("/api/file", get(file))
        .route("/api/viewed", post(viewed))
        .route("/api/watch", get(watch))
        .route("/health", get(move || health(identity.clone())))
        .fallback(assets::handler)
        .with_state(state)
}

/// Who is answering on this port.
///
/// It carries the whole entry rather than an empty 200 because a port that was
/// let go and taken by another server answers just as readily — what settles it
/// is the process id coming back the same.
async fn health(identity: ServerEntry) -> impl IntoResponse {
    Json(identity)
}

/// Everything the screen needs to draw itself, in one call.
async fn review(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.use_cases.review.execute() {
        Ok(snapshot) => Json(view::ReviewView::build(&snapshot)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct PathQuery {
    path: String,
}

async fn file(State(state): State<Arc<AppState>>, Query(q): Query<PathQuery>) -> impl IntoResponse {
    match state.use_cases.file_diff.execute(&q.path) {
        Ok(diff) => Json(diff).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct ViewedBody {
    path: String,
    viewed: bool,
}

async fn viewed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ViewedBody>,
) -> impl IntoResponse {
    let result = if body.viewed {
        state.use_cases.mark_viewed.execute(&body.path, &now())
    } else {
        state.use_cases.unmark_viewed.execute(&body.path)
    };
    match result {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

/// One-way channel: the page finds out that the repository moved without
/// polling for it.
async fn watch(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut rx = state.changes.subscribe();
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(kind) => yield Ok::<_, std::convert::Infallible>(Event::default().event(kind).data("")),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

/// Every failure here is the caller asking for something that is not under
/// review, which is a bad request rather than a broken server.
fn fail(e: Error) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, e.to_string()).into_response()
}

fn now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

#[cfg(test)]
pub(super) mod tests {
    //! Only what a request over the wire cannot reach.
    //!
    //! The routes themselves are exercised in `tests/server.rs`, against a real
    //! process: the browser talks to one, and the composition root and the
    //! store are part of what can break. What is left here needs a collaborator
    //! that fails, which no amount of driving the real binary can arrange.

    use super::*;
    use crate::cmd::ServerUseCases;
    use crate::progress::application::ProgressStore;
    use crate::testing::{FakeDiffSource, InMemoryProgressRepository};
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::Arc;
    use tower::ServiceExt as _;

    /// Who the server would say it is. Nothing under test here asks, but the
    /// router carries it.
    fn identity() -> ServerEntry {
        ServerEntry {
            port: 4600,
            pid: std::process::id(),
            repo: std::path::PathBuf::from("/repo"),
            branch: "feature/x".into(),
            base: "main".into(),
        }
    }

    /// The use cases over fakes, shared with the tests that start a real
    /// listener next door.
    pub(in crate::server) fn use_cases() -> ServerUseCases {
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::{GetFileDiff, GetReview};
        use crate::progress::application::{MarkViewed, UnmarkViewed};

        let paths = ["a.rs", "b.rs", "Cargo.lock"];
        let diffs = FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        let maps = crate::testing::services(
            FakeDiffSource::with_paths(&paths),
            Arc::new(crate::testing::InMemoryMapRepository::new()),
        );
        let progress = ProgressStore::new(
            Arc::new(InMemoryProgressRepository::default()),
            diffs.clone(),
        );
        ServerUseCases {
            review: GetReview::new(maps.versions, scope, progress.clone()),
            file_diff: GetFileDiff::new(diffs),
            mark_viewed: MarkViewed::new(progress.clone()),
            unmark_viewed: UnmarkViewed::new(progress),
        }
    }

    #[tokio::test]
    async fn a_review_that_cannot_be_assembled_is_a_bad_request_not_a_silent_empty_page() {
        // If the progress store is unreadable the screen must say so rather
        // than render as though nothing had been read.
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::{GetFileDiff, GetReview};
        use crate::progress::application::{MarkViewed, ProgressStore, UnmarkViewed};

        let paths = ["a.rs"];
        let diffs = FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        let maps = crate::testing::services(
            FakeDiffSource::with_paths(&paths),
            Arc::new(crate::testing::InMemoryMapRepository::new()),
        );
        // A map has to exist, or the request fails looking for one and never
        // reaches the collaborator this test is about.
        maps.editor
            .edit(|_| Ok::<_, crate::map::domain::MapError>(()))
            .unwrap();
        let broken = ProgressStore::new(
            Arc::new(crate::testing::BrokenProgressRepository),
            diffs.clone(),
        );
        let use_cases = ServerUseCases {
            review: GetReview::new(
                maps.versions,
                ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths))),
                broken.clone(),
            ),
            file_diff: GetFileDiff::new(diffs),
            mark_viewed: MarkViewed::new(broken.clone()),
            unmark_viewed: UnmarkViewed::new(broken),
        };
        let (state, _) = AppState::new(use_cases);

        let response = router(state, identity())
            .oneshot(
                Request::builder()
                    .uri("/api/review")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
