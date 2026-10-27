//! The lens catalog, read and changed from Settings.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use super::super::AppState;
use super::fail;
use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensGroup, LensId};

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
    let prefs = &state.preferences;
    let outcome = (|| {
        let mut catalog = None;
        if body.title.is_some() || body.prompt.is_some() {
            catalog = Some(prefs.update_lens.execute(
                &id,
                body.title.as_deref(),
                body.prompt.as_deref(),
            )?);
        }
        match body.enabled {
            Some(true) => catalog = Some(prefs.enable_lens.execute(&id)?),
            Some(false) => catalog = Some(prefs.disable_lens.execute(&id)?),
            None => {}
        }
        match catalog {
            Some(catalog) => Ok(catalog),
            None => prefs.list_lenses.execute(),
        }
    })();
    answer(outcome)
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
