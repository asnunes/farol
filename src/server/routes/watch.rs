//! The one-way channel that tells the page the repository moved.

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use tokio::sync::broadcast;

use super::super::AppState;

/// One-way channel: the page finds out that the repository moved without
/// polling for it.
///
/// The kind travels twice, as the event name and as the data. The data is not
/// redundant: a message whose data buffer is empty is dropped by the browser
/// rather than dispatched, so an event sent without one reaches curl and never
/// reaches a page.
pub(super) async fn watch(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut rx = state.changes.subscribe();
    let mut stopping = state.stopping.clone();
    let stream = async_stream::stream! {
        loop {
            tokio::select! {
                // Whichever comes first, and the second arm is why the process
                // can be stopped at all: this stream is what graceful shutdown
                // would otherwise wait on forever.
                nudge = rx.recv() => match nudge {
                    Ok(kind) => yield Ok::<_, std::convert::Infallible>(Event::default().event(&kind).data(&kind)),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                },
                _ = asked_to_stop(&mut stopping) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

/// Resolves once the process has been asked to stop.
///
/// `wait_for` rather than `changed`, so a connection that arrives after the
/// signal reads the state instead of waiting for a second one that is never
/// coming. It is a function of its own because the borrow the watch hands back
/// is not `Send`, and the stream it is selected in has to be.
pub(super) async fn asked_to_stop(stopping: &mut tokio::sync::watch::Receiver<bool>) {
    let _ = stopping.wait_for(|asked| *asked).await;
}
