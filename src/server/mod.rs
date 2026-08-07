//! The HTTP transport, and nothing else.
//!
//! Split by what each part is: `routes` turns requests into use-case calls,
//! `watch` pushes changes at the browser, `view` shapes what the screen reads,
//! `assets` serves the frontend. What is left here is starting and stopping.

mod assets;
mod routes;
pub mod view;
mod watch;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
        watch::spawn(config.git_dir, changes);
    }

    let app = routes::router(state);

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
