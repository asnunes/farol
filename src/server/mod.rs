//! The HTTP transport, and nothing else.
//!
//! Split by what each part is: `routes` turns requests into use-case calls,
//! `watch` pushes changes at the browser, `view` shapes what the screen reads,
//! `assets` serves the frontend. What is left here is starting and stopping.

mod assets;
pub mod detach;
mod health;
mod registry;
mod routes;
mod server_list;
pub mod view;
mod watch;

pub use registry::{Registry, ServerEntry};
pub use server_list::ServerList;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::cmd::ServerUseCases;
use crate::error::{Error, Result};
use crate::map::domain::ReviewMap;

/// Which port to listen on, and what to do if it is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Port {
    /// Whatever is free from `FIRST_PORT` upward. What you get when you did not
    /// ask for anything in particular.
    Free,
    /// This one and no other. Asking for a port and silently getting a
    /// different one would send you to the wrong tab.
    Exactly(u16),
    /// Whatever the OS hands out. For tests, which then read it back.
    Ephemeral,
}

/// Where the search starts. Above the usual dev-server crowd (3000, 5173, 8080)
/// so the first review of the day normally lands here.
pub const FIRST_PORT: u16 = 4600;

/// How far to look before giving up. Far enough for more reviews than anyone
/// opens at once, short enough that a machine with something odd going on says
/// so instead of scanning forever.
const PORTS_TO_TRY: u16 = 64;

pub struct ServeConfig {
    pub use_cases: ServerUseCases,
    pub map: ReviewMap,
    pub port: Port,
    pub open_browser: bool,
    pub watch: bool,
    pub git_dir: PathBuf,
    /// The working tree, for the list of what is running.
    pub repo: PathBuf,
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
    let registry = Registry::open()?;
    let (branch, base) = (config.map.branch.clone(), config.map.base.clone());
    let (state, changes) = AppState::new(config.use_cases, config.map);

    // One line of wiring, and the only one in this file with no test of its
    // own: `watch::spawn` is tested next door over a real directory, and the
    // route that carries its nudges is tested in `routes`. Driving both through
    // a live server needed a streaming client, and the timing-dependent test
    // that resulted was worse than saying so here.
    if config.watch {
        watch::spawn(config.git_dir, changes);
    }

    let listener = bind(config.port).await?;
    let addr = listener
        .local_addr()
        .map_err(|e| Error::msg(e.to_string()))?;
    let url = format!("http://{addr}");

    // Announced before anything else: whoever started this is waiting for a
    // port, and if it is a detached parent it is watching the registry for it.
    let entry = ServerEntry {
        port: addr.port(),
        pid: std::process::id(),
        repo: config.repo,
        branch,
        base,
    };
    registry.register(&entry)?;

    // The same entry the registry holds, so that asking the server who it is
    // and asking the file who it should be can be compared.
    let app = routes::router(state, entry.clone());

    println!("farol is reading at {url}");
    if config.open_browser {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }

    let served = axum::serve(listener, app)
        .with_graceful_shutdown(stopped())
        .await
        .map_err(|e| Error::msg(e.to_string()));

    // Taking itself out of the list is the whole reason shutdown is graceful.
    // A server that is cut down leaves its entry behind, and the next read of
    // the registry clears it.
    registry.deregister(entry.port)?;
    served
}

/// Take the port that was asked for, or find one.
async fn bind(port: Port) -> Result<tokio::net::TcpListener> {
    let one = async |p: u16| tokio::net::TcpListener::bind(("127.0.0.1", p)).await;

    match port {
        Port::Exactly(p) => one(p)
            .await
            .map_err(|e| Error::msg(format!("cannot listen on port {p}: {e}"))),
        Port::Ephemeral => one(0)
            .await
            .map_err(|e| Error::msg(format!("cannot listen: {e}"))),
        Port::Free => {
            for p in FIRST_PORT..FIRST_PORT.saturating_add(PORTS_TO_TRY) {
                if let Ok(listener) = one(p).await {
                    return Ok(listener);
                }
            }
            Err(Error::msg(format!(
                "no free port between {FIRST_PORT} and {} — pass --port to choose one",
                FIRST_PORT + PORTS_TO_TRY - 1
            )))
        }
    }
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

    #[tokio::test]
    async fn a_port_nobody_asked_for_is_the_first_free_one() {
        let listener = bind(Port::Free).await.unwrap();

        let port = listener.local_addr().unwrap().port();
        assert!(
            (FIRST_PORT..FIRST_PORT + PORTS_TO_TRY).contains(&port),
            "{port} is outside the range farol searches"
        );
    }

    #[tokio::test]
    async fn the_search_steps_over_ports_that_are_taken() {
        // Two reviews open at once is the ordinary case — one per worktree.
        let first = bind(Port::Free).await.unwrap();
        let second = bind(Port::Free).await.unwrap();

        assert_ne!(
            first.local_addr().unwrap().port(),
            second.local_addr().unwrap().port()
        );
    }

    #[tokio::test]
    async fn a_port_that_was_asked_for_is_the_one_you_get_or_none() {
        // Quietly serving somewhere else would send the reviewer to a tab with
        // nothing in it, or worse, to somebody else's review.
        let held = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let taken = held.local_addr().unwrap().port();

        let err = bind(Port::Exactly(taken)).await.unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains(&taken.to_string()), "{msg}");
        assert!(msg.contains("cannot listen"), "{msg}");
    }

    #[tokio::test]
    async fn asking_for_zero_lets_the_operating_system_choose() {
        let listener = bind(Port::Ephemeral).await.unwrap();

        assert_ne!(listener.local_addr().unwrap().port(), 0);
    }
}
