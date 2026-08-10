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
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::Position;
    use std::time::Duration;

    fn config(port: u16, watch: bool) -> ServeConfig {
        let mut map = ReviewMap::new("feature/x", "main", "head");
        map.add_block(&crate::testing::slug("core"), "t", "c", Position::End)
            .unwrap();
        ServeConfig {
            use_cases: routes::tests::use_cases(),
            map,
            port,
            open_browser: false,
            watch,
            git_dir: std::env::temp_dir(),
        }
    }

    #[test]
    fn the_state_hands_out_the_channel_the_watcher_writes_to() {
        // The watcher and the SSE route have to meet on the same channel, or
        // the browser is told nothing and never reloads.
        let (state, changes) =
            AppState::new(routes::tests::use_cases(), ReviewMap::new("b", "m", "h"));

        let mut rx = state.changes.subscribe();
        changes.send("map".into()).unwrap();

        assert_eq!(rx.try_recv().unwrap(), "map");
    }

    #[tokio::test]
    async fn the_server_listens_and_answers_on_the_port_it_was_given() {
        // Port 0 asks the OS for a free one, which is also how `--port 0`
        // behaves for the reviewer.
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let handle = tokio::spawn(async move { serve(config(port, false)).await });
        let url = format!("http://127.0.0.1:{port}/api/review");

        let body = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let out = tokio::process::Command::new("curl")
                    .args(["-sf", &url])
                    .output()
                    .await
                    .unwrap();
                if out.status.success() {
                    return String::from_utf8_lossy(&out.stdout).into_owned();
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the server should have come up");

        assert!(body.contains("feature/x"), "{body}");
        handle.abort();
    }

    #[tokio::test]
    async fn a_port_already_taken_is_reported_rather_than_swallowed() {
        // Two farols on one port is an ordinary mistake, and the message has
        // to say which port so the reviewer can pick another.
        let held = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = held.local_addr().unwrap().port();

        let err = serve(config(port, false)).await.unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains(&port.to_string()), "{msg}");
        assert!(msg.contains("cannot listen"), "{msg}");
    }
}
