use crate::diff::domain::ReviewPath;
use crate::map::application::MapService;
use crate::map::domain::{Position, ReviewMap, Slug};
use crate::shared::error::Result;

/// Open a block and, optionally, seed it with files that need no note.
#[derive(Clone)]
pub struct AddBlock {
    maps: MapService,
}

impl AddBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        title: &str,
        context: &str,
        position: Position,
        paths: &[ReviewPath],
    ) -> Result<ReviewMap> {
        self.maps.edit(|map| {
            map.add_block(slug, title, context, position)?;
            for path in paths {
                map.add_file(slug, path.as_str(), None, None)?;
            }
            Ok(())
        })
    }
}

/// Rewrite a block's title or the text that explains why it exists.
#[derive(Clone)]
pub struct UpdateBlock {
    maps: MapService,
}

impl UpdateBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        title: Option<String>,
        context: Option<String>,
    ) -> Result<ReviewMap> {
        self.maps.edit(|map| map.update_block(slug, title, context))
    }
}

/// Drop a block. Its line notes survive as orphans, because the prose may still
/// be worth moving somewhere else.
#[derive(Clone)]
pub struct RemoveBlock {
    maps: MapService,
}

impl RemoveBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_block(slug))
    }
}

/// Move a block in the reading order.
#[derive(Clone)]
pub struct MoveBlock {
    maps: MapService,
}

impl MoveBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, position: Position) -> Result<ReviewMap> {
        self.maps.edit(|map| map.move_block(slug, position))
    }
}
