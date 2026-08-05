use crate::diff::domain::ReviewPath;
use crate::map::application::MapService;
use crate::map::domain::{ReviewMap, Slug};
use crate::shared::error::Result;

/// Mark a file as safe to read diagonally, optionally next to the block that
/// caused it to change.
#[derive(Clone)]
pub struct AddSkim {
    maps: MapService,
}

impl AddSkim {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        path: &ReviewPath,
        reason: &str,
        block: Option<Slug>,
    ) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.add_skim(path.as_str(), reason, block))
    }
}

#[derive(Clone)]
pub struct RemoveSkim {
    maps: MapService,
}

impl RemoveSkim {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, path: &ReviewPath) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_skim(path.as_str()))
    }
}
