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

use super::{AppState, assets, view};
use crate::shared::error::Error;

/// The whole HTTP surface. Everything it serves arrives injected, so the routes
/// can be exercised over fakes with nothing listening on a port.
pub(super) fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/review", get(review))
        .route("/api/file", get(file))
        .route("/api/viewed", post(viewed))
        .route("/api/watch", get(watch))
        .fallback(assets::handler)
        .with_state(state)
}

/// Everything the screen needs to draw itself, in one call.
async fn review(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let map = state.map.lock().unwrap().clone();
    match state.use_cases.review.execute(&map) {
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
mod tests {
    use super::*;
    use crate::cmd::ServerUseCases;
    use crate::map::domain::ReviewMap;
    use crate::map::domain::{LineRange, Position};
    use crate::progress::application::ProgressStore;
    use crate::testing::{FakeDiffSource, InMemoryProgressRepository, slug};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    fn mapped() -> ReviewMap {
        let mut map = ReviewMap::new("feature/x", "main", "head");
        map.add_block(&slug("core"), "The change", "why it exists", Position::End)
            .unwrap();
        map.add_file(&slug("core"), "a.rs", Some("worth knowing".into()), None)
            .unwrap();
        map.add_line_note(
            &slug("core"),
            "a.rs",
            LineRange::new(4, 8).unwrap(),
            "local point",
        )
        .unwrap();
        map.add_file(&slug("core"), "b.rs", None, None).unwrap();
        map.add_skim("Cargo.lock", "generated", None).unwrap();
        map
    }

    /// Wire the routes over fakes. Nothing listens on a port and nothing
    /// touches disk.
    fn app() -> (Router, ProgressStore, broadcast::Sender<String>) {
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::{GetFileDiff, GetReview};
        use crate::progress::application::{MarkViewed, UnmarkViewed};

        let source = Arc::new(FakeDiffSource::with_paths(&["a.rs", "b.rs", "Cargo.lock"]));
        let diffs = FileDiffs::new(source.clone());
        let scope = ReviewScope::new(source.clone());
        let maps = crate::testing::services(
            FakeDiffSource::with_paths(&["a.rs", "b.rs", "Cargo.lock"]),
            Arc::new(crate::testing::InMemoryMapRepository::new()),
        );
        let progress = ProgressStore::new(
            Arc::new(InMemoryProgressRepository::default()),
            diffs.clone(),
        );

        let use_cases = ServerUseCases {
            review: GetReview::new(maps.versions, scope, progress.clone()),
            file_diff: GetFileDiff::new(diffs),
            mark_viewed: MarkViewed::new(progress.clone()),
            unmark_viewed: UnmarkViewed::new(progress.clone()),
        };
        let (state, changes) = AppState::new(use_cases, mapped());
        (router(state), progress, changes)
    }

    async fn json(app: &Router, uri: &str) -> Value {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "GET {uri} should succeed"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn post_viewed(app: &Router, path: &str, viewed: bool) -> StatusCode {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/viewed")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "path": path, "viewed": viewed }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn the_review_endpoint_serves_the_map_in_reading_order() {
        let (app, _, _) = app();
        let body = json(&app, "/api/review").await;

        assert_eq!(body["branch"], "feature/x");
        assert_eq!(body["blocks"][0]["slug"], "core");
        assert_eq!(body["blocks"][0]["title"], "The change");
        assert_eq!(body["blocks"][0]["context"], "why it exists");

        let files = body["blocks"][0]["files"].as_array().unwrap();
        assert_eq!(files[0]["path"], "a.rs");
        assert_eq!(files[0]["notes"][0]["text"], "worth knowing");
        assert_eq!(files[0]["lineNotes"][0]["from"], 4);
        assert_eq!(body["looseSkim"][0]["path"], "Cargo.lock");
    }

    #[tokio::test]
    async fn the_file_endpoint_returns_a_diff_and_refuses_a_path_outside_the_review() {
        let (app, _, _) = app();
        let body = json(&app, "/api/file?path=a.rs").await;
        assert_eq!(body["path"], "a.rs");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/file?path=nowhere.rs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn marking_a_file_read_is_visible_on_the_next_review() {
        let (app, _, _) = app();
        assert_eq!(json(&app, "/api/review").await["viewedFiles"], 0);

        assert_eq!(
            post_viewed(&app, "a.rs", true).await,
            StatusCode::NO_CONTENT
        );
        assert_eq!(json(&app, "/api/review").await["viewedFiles"], 1);

        assert_eq!(
            post_viewed(&app, "a.rs", false).await,
            StatusCode::NO_CONTENT
        );
        assert_eq!(json(&app, "/api/review").await["viewedFiles"], 0);
    }

    #[tokio::test]
    async fn marking_writes_through_to_the_repository() {
        // The count on screen must come from stored state, not from something
        // the handler kept in memory.
        let (app, progress, _) = app();
        post_viewed(&app, "b.rs", true).await;
        assert!(progress.load().unwrap().is_current("b.rs", "hash-of-b.rs"));
    }

    #[tokio::test]
    async fn marking_a_path_outside_the_review_is_refused() {
        let (app, _, _) = app();
        assert_eq!(
            post_viewed(&app, "nowhere.rs", true).await,
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn the_watch_channel_delivers_what_the_watcher_publishes() {
        // The SSE route is a thin wrapper over this broadcast; if a subscriber
        // gets the message, the page finds out the repository moved.
        let (_, _, changes) = app();
        let mut rx = changes.subscribe();
        changes.send("map".to_string()).unwrap();

        let received = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("the channel should deliver promptly")
            .unwrap();
        assert_eq!(received, "map");
    }
}
