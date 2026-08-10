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
use crate::error::{Error, Result};
use crate::map::domain::ReviewMap;

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

    // One line of wiring, and the only one in this file with no test of its
    // own: `watch::spawn` is tested next door over a real directory, and the
    // route that carries its nudges is tested in `routes`. Driving both through
    // a live server needed a streaming client, and the timing-dependent test
    // that resulted was worse than saying so here.
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
        .with_graceful_shutdown(stopped())
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Ctrl-C, or a `kill` from whatever started us.
///
/// Returning from `serve` rather than being cut down mid-flight is what lets
/// the process run its exit handlers — which is also how the integration tests
/// get anything back from it.
async fn stopped() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    {
        let mut term =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(_) => return interrupt.await,
            };
        tokio::select! {
            _ = interrupt => {}
            _ = term.recv() => {}
        }
    }

    #[cfg(not(unix))]
    interrupt.await;
}

#[cfg(test)]
mod tests {
    //! Starting and stopping is exercised in `tests/server.rs`, against the
    //! real binary. What is left here is the wiring between the state and the
    //! channel, which no request can observe directly.

    use super::*;

    #[test]
    fn the_state_hands_out_the_channel_the_watcher_writes_to() {
        // The watcher and the SSE route have to meet on the same channel, or
        // the browser is told nothing and never reloads.
        let (state, changes) = AppState::new(
            routes::tests::use_cases(),
            ReviewMap::new("feature/x", "main", "head"),
        );

        let mut rx = state.changes.subscribe();
        changes.send("map".into()).unwrap();

        assert_eq!(rx.try_recv().unwrap(), "map");
    }
}
