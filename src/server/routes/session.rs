//! Who is answering on this port, and pointing it at another invocation.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;

use super::super::{AppState, SessionConfig};
use super::{away, fail};

/// Who is answering on this port.
///
/// It carries the whole entry rather than an empty 200 because a port that was
/// let go and taken by another server answers just as readily — what settles it
/// is the process id coming back the same.
pub(super) async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.session.identity())
}

pub(super) async fn configure(
    State(state): State<Arc<AppState>>,
    Json(config): Json<SessionConfig>,
) -> impl IntoResponse {
    let session = state.session.clone();
    match away(move || session.configure(config)).await {
        Ok(identity) => {
            let _ = state.changes.send("map".to_string());
            // The session may have changed, and with it the name on the Send
            // button, which the page reads off the comments.
            let _ = state.changes.send("comments".to_string());
            Json(identity).into_response()
        }
        Err(e) => fail(e),
    }
}
