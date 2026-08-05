mod assets;
pub mod view;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::cmd::ServerUseCases;
use crate::map::domain::ReviewMap;
use crate::shared::error::{Error, Result};

pub struct ServeConfig {
    pub use_cases: ServerUseCases,
    pub map: ReviewMap,
    pub port: u16,
    pub open_browser: bool,
    pub watch: bool,
    pub git_dir: PathBuf,
}

struct AppState {
    /// Transport holds use cases and nothing else: the routes translate HTTP
    /// into a call and back, and own no business logic of their own.
    use_cases: ServerUseCases,
    map: Mutex<ReviewMap>,
    changes: broadcast::Sender<String>,
}

impl AppState {
    fn new(use_cases: ServerUseCases, map: ReviewMap) -> (Arc<Self>, broadcast::Sender<String>) {
        let (changes, _) = broadcast::channel(16);
        let state = Arc::new(Self {
            use_cases,
            map: Mutex::new(map),
            changes: changes.clone(),
        });
        (state, changes)
    }
}

/// The routes, separated from the listener so they can be driven directly in a
/// test — which is the whole reason the collaborators are injected.
fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/review", get(review))
        .route("/api/file", get(file))
        .route("/api/viewed", post(viewed))
        .route("/api/watch", get(watch))
        .fallback(assets::handler)
        .with_state(state)
}

/// Owns the HTTP surface. Everything it serves arrives injected, so the routes
/// can be exercised over fakes.
pub struct Server {
    config: ServeConfig,
}

impl Server {
    pub fn new(config: ServeConfig) -> Self {
        Self { config }
    }

    pub fn run(self) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| Error::msg(e.to_string()))?;
        runtime.block_on(async move { serve(self.config).await })
    }
}

async fn serve(config: ServeConfig) -> Result<()> {
    let (state, changes) = AppState::new(config.use_cases, config.map);

    if config.watch {
        spawn_watcher(config.git_dir, changes);
    }

    let app = router(state);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", config.port))
        .await
        .map_err(|e| Error::msg(format!("cannot listen on port {}: {e}", config.port)))?;
    let addr = listener
        .local_addr()
        .map_err(|e| Error::msg(e.to_string()))?;
    let url = format!("http://{addr}");

    println!("farol is reading at {url}");
    if config.open_browser {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

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

/// Watch the git dir rather than the worktree: commits, checkouts and the
/// skill writing a map all land here, while a big working tree would flood us
/// with noise from builds.
fn spawn_watcher(git_dir: PathBuf, tx: broadcast::Sender<String>) {
    std::thread::spawn(move || {
        use notify::{RecursiveMode, Watcher};

        let (raw_tx, raw_rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(raw_tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("warning: cannot watch the repository: {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&git_dir, RecursiveMode::Recursive) {
            eprintln!("warning: cannot watch {}: {e}", git_dir.display());
            return;
        }

        let mut last = std::time::Instant::now();
        for event in raw_rx {
            let Ok(event) = event else { continue };
            let touched_map = event
                .paths
                .iter()
                .any(|p| p.to_string_lossy().contains("/farol/"));
            let touched_head = event
                .paths
                .iter()
                .any(|p| p.ends_with("HEAD") || p.to_string_lossy().contains("/refs/"));

            if !touched_map && !touched_head {
                continue;
            }
            // Git rewrites several files per operation; one nudge is enough.
            if last.elapsed() < std::time::Duration::from_millis(300) {
                continue;
            }
            last = std::time::Instant::now();
            let kind = if touched_map { "map" } else { "head" };
            let _ = tx.send(kind.to_string());
        }
    });
}

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
        map.add_skim("go.sum", "generated", None).unwrap();
        map
    }

    /// Wire the routes over fakes. Nothing listens on a port and nothing
    /// touches disk.
    fn app() -> (Router, ProgressStore, broadcast::Sender<String>) {
        use crate::diff::application::DiffService;
        use crate::map::application::{GetFileDiff, GetReview, MapService};
        use crate::progress::application::{MarkViewed, UnmarkViewed};

        let diff = DiffService::new(Arc::new(FakeDiffSource::with_paths(&[
            "a.rs", "b.rs", "go.sum",
        ])));
        let maps = MapService::new(
            diff.clone(),
            Arc::new(crate::testing::InMemoryMapRepository::new()),
        );
        let progress = ProgressStore::new(
            Arc::new(InMemoryProgressRepository::default()),
            diff.clone(),
        );

        let use_cases = ServerUseCases {
            review: GetReview::new(maps, progress.clone()),
            file_diff: GetFileDiff::new(diff),
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
        assert_eq!(body["looseSkim"][0]["path"], "go.sum");
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
