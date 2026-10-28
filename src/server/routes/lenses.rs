//! The lens catalog, read and changed from Settings.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use super::super::AppState;
use super::{away, fail};
use crate::cmd::ServerUseCases;
use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensGroup, LensId, LensTarget};

pub(super) async fn list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    answer(state.preferences.list_lenses.execute())
}

pub(super) async fn create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> impl IntoResponse {
    let group = match LensGroup::parse(&body.group) {
        Ok(group) => group,
        Err(e) => return fail(e.into()),
    };
    answer(
        state
            .preferences
            .add_lens
            .execute(&body.title, group, &body.prompt),
    )
}

/// Title, prompt and on/off, each only when sent: the panel saves a switch on
/// its own and the words on their own.
pub(super) async fn save(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<SaveBody>,
) -> impl IntoResponse {
    let id = match LensId::parse(&id) {
        Ok(id) => id,
        Err(e) => return fail(e.into()),
    };
    answer(state.preferences.update_lens.execute(
        &id,
        body.title.as_deref(),
        body.prompt.as_deref(),
        body.enabled,
    ))
}

pub(super) async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match LensId::parse(&id) {
        Ok(id) => answer(state.preferences.delete_lens.execute(&id)),
        Err(e) => fail(e.into()),
    }
}

/// The reader marking a lens as seen, or taking the mark back.
pub(super) async fn seen(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<SeenBody>,
) -> impl IntoResponse {
    let id = match LensId::parse(&body.lens) {
        Ok(id) => id,
        Err(e) => return fail(e.into()),
    };
    let at = super::review::now();
    match away(move || cases.see_lens.execute(&id, &body.target, body.seen, &at)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

/// Every change answers with the catalog as it now stands, so the panel
/// redraws from the server rather than guessing.
fn answer(outcome: Result<LensCatalog>) -> axum::response::Response {
    match outcome {
        Ok(catalog) => Json(CatalogView::from(&catalog)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SeenBody {
    lens: String,
    target: LensTarget,
    seen: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateBody {
    title: String,
    group: String,
    prompt: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SaveBody {
    title: Option<String>,
    prompt: Option<String>,
    enabled: Option<bool>,
}

#[derive(Serialize)]
struct CatalogView {
    lenses: Vec<LensRow>,
}

impl From<&LensCatalog> for CatalogView {
    fn from(catalog: &LensCatalog) -> Self {
        Self {
            lenses: catalog
                .all()
                .into_iter()
                .map(|entry| LensRow {
                    id: entry.definition.id.to_string(),
                    title: entry.definition.title,
                    group: entry.definition.group.name(),
                    prompt: entry.definition.prompt,
                    enabled: entry.enabled,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct LensRow {
    id: String,
    title: String,
    group: &'static str,
    prompt: String,
    enabled: bool,
}
