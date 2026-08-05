use crate::diff::domain::ReviewPath;
use crate::map::application::MapService;
use crate::map::domain::{ReviewMap, Slug};
use crate::shared::error::Result;

/// Put a file into a block, at a chosen place in its reading order.
#[derive(Clone)]
pub struct AddFile {
    maps: MapService,
}

impl AddFile {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        path: &ReviewPath,
        note: Option<String>,
        after: Option<&ReviewPath>,
    ) -> Result<ReviewMap> {
        let after = after.map(ReviewPath::as_str);
        self.maps
            .edit(|map| map.add_file(slug, path.as_str(), note, after))
    }
}

/// Rewrite the note that explains what a file contributes to its block.
#[derive(Clone)]
pub struct UpdateFile {
    maps: MapService,
}

impl UpdateFile {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath, note: String) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.update_file(slug, path.as_str(), Some(note)))
    }
}

/// Take a file out of a block — because the change to it was reverted, or it
/// turned out to belong elsewhere.
#[derive(Clone)]
pub struct RemoveFile {
    maps: MapService,
}

impl RemoveFile {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_file(slug, path.as_str()))
    }
}
