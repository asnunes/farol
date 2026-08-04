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

use crate::diff::domain::DiffSource;
use crate::map::domain::ReviewMap;
use crate::progress::domain::ProgressRepository;
use crate::shared::error::{Error, Result};

pub struct ServeConfig {
    pub source: Arc<dyn DiffSource>,
    pub progress: Arc<dyn ProgressRepository>,
    pub map: ReviewMap,
    pub port: u16,
    pub open_browser: bool,
    pub watch: bool,
    pub git_dir: PathBuf,
}

struct AppState {
    source: Arc<dyn DiffSource>,
    progress: Arc<dyn ProgressRepository>,
    map: Mutex<ReviewMap>,
    changes: broadcast::Sender<String>,
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
    let (tx, _) = broadcast::channel(16);

    let state = Arc::new(AppState {
        source: config.source,
        progress: config.progress,
        map: Mutex::new(config.map),
        changes: tx.clone(),
    });

    if config.watch {
        spawn_watcher(config.git_dir, tx.clone());
    }

    let app = Router::new()
        .route("/api/review", get(review))
        .route("/api/file", get(file))
        .route("/api/viewed", post(viewed))
        .route("/api/watch", get(watch))
        .fallback(assets::handler)
        .with_state(state);

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
    let progress = match state.progress.load() {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    let map = state.map.lock().unwrap().clone();
    match view::ReviewView::build(&map, state.source.as_ref(), &progress) {
        Ok(v) => Json(v).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct PathQuery {
    path: String,
}

async fn file(State(state): State<Arc<AppState>>, Query(q): Query<PathQuery>) -> impl IntoResponse {
    match state.source.file_diff(&q.path) {
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
    let service = crate::progress::application::ProgressService::new(
        state.progress.as_ref(),
        state.source.as_ref(),
    );
    let result = if body.viewed {
        service.mark(&body.path, &now())
    } else {
        service.unmark(&body.path)
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
