//! Changing the map that applies now, and throwing a version away.
//!
//! Every write goes through here, so the load-change-store sequence exists in
//! one place: a use case says what to change, not how to persist it.

use std::sync::Arc;

use super::derivation::MapDerivation;
use super::versions::MapVersions;
use crate::map::domain::{MapRepository, ReviewMap};
use crate::shared::error::Result;

pub enum ResetOutcome {
    /// Deleted; the named version is current again, or nothing is.
    Deleted {
        fell_back_to: Option<String>,
    },
    NothingToDelete,
}

#[derive(Clone)]
pub struct MapEditor {
    /// An edit derives first, so writing to a commit that has no version yet
    /// starts one rather than failing.
    derivation: MapDerivation,
    versions: MapVersions,
    repo: Arc<dyn MapRepository>,
}

impl MapEditor {
    pub fn new(
        derivation: MapDerivation,
        versions: MapVersions,
        repo: Arc<dyn MapRepository>,
    ) -> Self {
        Self {
            derivation,
            versions,
            repo,
        }
    }

    /// Load the version for where we are, apply `f`, store it back.
    pub fn edit<F>(&self, f: F) -> Result<ReviewMap>
    where
        F: FnOnce(&mut ReviewMap) -> Result<()>,
    {
        let mut map = self.derivation.derive()?.map;
        f(&mut map)?;
        self.repo.save(&map)?;
        Ok(map)
    }

    /// Drop the version for where we are, so the previous one applies again.
    pub fn reset(&self) -> Result<ResetOutcome> {
        let target = self.versions.target()?;
        if self.versions.at_target()?.is_none() {
            return Ok(ResetOutcome::NothingToDelete);
        }
        self.repo.delete(&target)?;
        Ok(ResetOutcome::Deleted {
            fell_back_to: self.versions.current()?.map(|m| m.generated_at),
        })
    }
}
