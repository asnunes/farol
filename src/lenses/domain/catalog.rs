//! The lenses in effect: the ones farol ships, as the user left them, then the
//! ones the user made.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::built_in::{BUILT_IN, built_in};
use super::{LensDefinition, LensError, LensGroup, LensId};
use crate::error::Result;

/// Only what the user changed is stored, as with settings: a lens farol ships
/// that nobody edited follows the prompt a later version ships, and switching
/// one on or off is kept apart from editing it so that it does not freeze the
/// prompt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LensCatalog {
    /// Lenses the user made, and shipped ones they edited.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    lenses: Vec<LensDefinition>,
    /// Lenses switched away from how they ship, or from how they were made.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    switched: BTreeMap<LensId, bool>,
    /// Shipped lenses the user deleted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    deleted: Vec<LensId>,
}

impl LensCatalog {
    /// Every lens, in the order the screen lists them: shipped first, then the
    /// user's, each group keeping its own order.
    pub fn all(&self) -> Vec<LensEntry> {
        let shipped = BUILT_IN
            .iter()
            .filter(|lens| !self.deleted.iter().any(|id| id.as_str() == lens.id))
            .map(|lens| {
                let shipped = lens.definition();
                let definition = self.stored(&shipped.id).cloned().unwrap_or(shipped);
                LensEntry {
                    enabled: self
                        .switched
                        .get(&definition.id)
                        .copied()
                        .unwrap_or(lens.enabled),
                    built_in: true,
                    definition,
                }
            });
        let made = self
            .lenses
            .iter()
            .filter(|lens| built_in(lens.id.as_str()).is_none())
            .map(|lens| LensEntry {
                enabled: self.switched.get(&lens.id).copied().unwrap_or(true),
                built_in: false,
                definition: lens.clone(),
            });
        shipped.chain(made).collect()
    }

    /// The lenses the session is asked to write.
    pub fn enabled(&self) -> Vec<LensDefinition> {
        self.all()
            .into_iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.definition)
            .collect()
    }

    pub fn require(&self, id: &LensId) -> std::result::Result<LensEntry, LensError> {
        let all = self.all();
        all.iter()
            .find(|entry| &entry.definition.id == id)
            .cloned()
            .ok_or_else(|| LensError::UnknownLens {
                id: id.to_string(),
                known: all.iter().map(|e| e.definition.id.to_string()).collect(),
            })
    }

    /// A new lens, its id made from the title and never one already taken.
    pub fn add(
        &mut self,
        title: &str,
        group: LensGroup,
        prompt: &str,
    ) -> std::result::Result<LensDefinition, LensError> {
        let (title, prompt) = words(title, prompt)?;
        let base = LensId::from_title(&title);
        let taken = |id: &LensId| {
            built_in(id.as_str()).is_some() || self.lenses.iter().any(|lens| &lens.id == id)
        };
        let id = if taken(&base) {
            (2..)
                .map(|n| {
                    LensId::parse(&format!("{base}-{n}")).expect("a suffix keeps the id valid")
                })
                .find(|id| !taken(id))
                .expect("some suffix is always free")
        } else {
            base
        };
        let lens = LensDefinition {
            id,
            title,
            group,
            prompt,
        };
        self.lenses.push(lens.clone());
        Ok(lens)
    }

    /// Change a lens's words. A shipped lens edited back to what farol ships is
    /// forgotten rather than stored, so it follows the next default again.
    pub fn update(
        &mut self,
        id: &LensId,
        title: Option<&str>,
        prompt: Option<&str>,
    ) -> std::result::Result<(), LensError> {
        let current = self.require(id)?.definition;
        let (title, prompt) = words(
            title.unwrap_or(&current.title),
            prompt.unwrap_or(&current.prompt),
        )?;
        let edited = LensDefinition {
            title,
            prompt,
            ..current
        };
        let shipped = built_in(id.as_str()).map(|lens| lens.definition());
        let at = self.lenses.iter().position(|lens| &lens.id == id);
        match (at, shipped.as_ref() == Some(&edited)) {
            (Some(at), true) => {
                self.lenses.remove(at);
            }
            (Some(at), false) => self.lenses[at] = edited,
            (None, false) => self.lenses.push(edited),
            (None, true) => {}
        }
        Ok(())
    }

    pub fn switch(&mut self, id: &LensId, enabled: bool) -> std::result::Result<(), LensError> {
        self.require(id)?;
        let default = built_in(id.as_str())
            .map(|lens| lens.enabled)
            .unwrap_or(true);
        if enabled == default {
            self.switched.remove(id);
        } else {
            self.switched.insert(id.clone(), enabled);
        }
        Ok(())
    }

    /// Gone for good, shipped or made.
    pub fn delete(&mut self, id: &LensId) -> std::result::Result<(), LensError> {
        self.require(id)?;
        self.lenses.retain(|lens| &lens.id != id);
        self.switched.remove(id);
        if built_in(id.as_str()).is_some() {
            self.deleted.push(id.clone());
        }
        Ok(())
    }

    fn stored(&self, id: &LensId) -> Option<&LensDefinition> {
        self.lenses.iter().find(|lens| &lens.id == id)
    }
}

/// A lens as it is in effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LensEntry {
    pub definition: LensDefinition,
    pub enabled: bool,
    pub built_in: bool,
}

pub trait LensCatalogRepository: Send + Sync {
    fn read(&self) -> Result<LensCatalog>;
    fn write(&self, catalog: &LensCatalog) -> Result<()>;
}

fn words(title: &str, prompt: &str) -> std::result::Result<(String, String), LensError> {
    let (title, prompt) = (title.trim(), prompt.trim());
    if title.is_empty() {
        return Err(LensError::NoTitle);
    }
    if prompt.is_empty() {
        return Err(LensError::NoPrompt);
    }
    Ok((title.to_string(), prompt.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(raw: &str) -> LensId {
        LensId::parse(raw).unwrap()
    }

    #[test]
    fn a_catalog_nobody_touched_is_what_farol_ships_and_stores_nothing() {
        let catalog = LensCatalog::default();

        let on: Vec<_> = catalog
            .enabled()
            .into_iter()
            .map(|l| l.id.to_string())
            .collect();
        assert_eq!(on, vec!["overview", "risk", "evidence", "summary", "tests"]);
        assert_eq!(
            serde_json::to_value(&catalog).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn switching_a_shipped_lens_does_not_freeze_its_prompt() {
        let mut catalog = LensCatalog::default();

        catalog.switch(&id("vocabulary"), true).unwrap();
        catalog.switch(&id("overview"), false).unwrap();

        assert_eq!(
            serde_json::to_value(&catalog).unwrap(),
            serde_json::json!({"switched": {"overview": false, "vocabulary": true}}),
            "a prompt shipped later has to reach whoever only switched the lens"
        );
        catalog.switch(&id("overview"), true).unwrap();
        assert!(
            !serde_json::to_value(&catalog).unwrap()["switched"]
                .as_object()
                .unwrap()
                .contains_key("overview")
        );
    }

    #[test]
    fn a_shipped_lens_edited_back_to_how_it_ships_is_forgotten() {
        let mut catalog = LensCatalog::default();
        let shipped = catalog.require(&id("risk")).unwrap().definition;

        catalog
            .update(&id("risk"), None, Some("Only the risk."))
            .unwrap();
        assert_eq!(
            catalog.require(&id("risk")).unwrap().definition.prompt,
            "Only the risk."
        );
        catalog
            .update(&id("risk"), None, Some(&shipped.prompt))
            .unwrap();

        assert_eq!(catalog, LensCatalog::default());
    }

    #[test]
    fn a_deleted_lens_is_gone_for_good() {
        let mut catalog = LensCatalog::default();
        let made = catalog
            .add("Migrations", LensGroup::File, "What each migration does.")
            .unwrap();

        catalog.delete(&id("risk")).unwrap();
        catalog.delete(&made.id).unwrap();

        assert!(catalog.require(&id("risk")).is_err());
        assert!(catalog.require(&made.id).is_err());
    }

    #[test]
    fn a_new_lens_never_takes_an_id_already_used() {
        let mut catalog = LensCatalog::default();

        let first = catalog.add("Tests", LensGroup::Block, "x").unwrap();
        let second = catalog.add("Tests", LensGroup::Block, "x").unwrap();

        assert_eq!(first.id.as_str(), "tests-2", "a shipped id is taken too");
        assert_eq!(second.id.as_str(), "tests-3");
    }

    #[test]
    fn a_lens_without_words_is_refused() {
        let mut catalog = LensCatalog::default();
        assert!(catalog.add(" ", LensGroup::Pr, "x").is_err());
        assert!(catalog.add("Title", LensGroup::Pr, " ").is_err());
    }

    #[test]
    fn an_unknown_lens_is_refused_naming_the_ones_there_are() {
        let error = LensCatalog::default()
            .require(&id("risks"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("risk"), "{error}");
        assert!(error.contains("farol lens list"), "{error}");
    }
}
