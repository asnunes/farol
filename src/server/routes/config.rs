//! The user's preferences, read and written from the Settings panel.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use super::super::AppState;
use super::fail;
use crate::config::presentation::ConfigView;

pub(super) async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.preferences.get.execute() {
        Ok(config) => Json(ConfigView::from(config)).into_response(),
        Err(error) => fail(error),
    }
}

pub(super) async fn set_config(
    State(state): State<Arc<AppState>>,
    Json(setting): Json<SettingBody>,
) -> impl IntoResponse {
    let key = match crate::config::domain::SettingKey::parse(&setting.key) {
        Ok(key) => key,
        Err(error) => return fail(error.into()),
    };
    match state.preferences.set_setting.execute(key, setting.value) {
        Ok(config) => Json(ConfigView::from(config)).into_response(),
        Err(error) => fail(error),
    }
}

/// Back to the default, answering with the configuration as it now stands so
/// the page redraws the field from the server rather than guessing.
pub(super) async fn reset_config(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let key = match crate::config::domain::SettingKey::parse(&key) {
        Ok(key) => key,
        Err(error) => return fail(error.into()),
    };
    match state.preferences.reset_setting.execute(key) {
        Ok(config) => Json(ConfigView::from(config)).into_response(),
        Err(error) => fail(error),
    }
}

/// The value as JSON wrote it. Whether it suits the key is the setting's to
/// decide, so a mismatch is refused there and names what the key takes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SettingBody {
    key: String,
    value: crate::config::domain::SettingValue,
}
