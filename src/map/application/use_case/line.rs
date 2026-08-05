use crate::diff::domain::ReviewPath;
use crate::map::application::MapService;
use crate::map::domain::{LineRange, ReviewMap, Slug};
use crate::shared::error::Result;

/// Pin a note to a span of lines. The span is checked against the file here,
/// because a note past the end would render nowhere.
#[derive(Clone)]
pub struct AddLineNote {
    maps: MapService,
}

impl AddLineNote {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        path: &ReviewPath,
        range: LineRange,
        note: String,
    ) -> Result<ReviewMap> {
        range.require_within(path)?;
        self.maps
            .edit(|map| map.add_line_note(slug, path.as_str(), range, note))
    }
}

/// Rewrite a note whose span did not move.
#[derive(Clone)]
pub struct UpdateLineNote {
    maps: MapService,
}

impl UpdateLineNote {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        path: &ReviewPath,
        range: LineRange,
        note: String,
    ) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.update_line_note(slug, path.as_str(), range, note))
    }
}

#[derive(Clone)]
pub struct RemoveLineNote {
    maps: MapService,
}

impl RemoveLineNote {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath, range: LineRange) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.remove_line_note(slug, path.as_str(), range))
    }
}

/// Bring a deactivated note back where its code moved to.
///
/// Only the new span is checked against the file; the old one is a key into the
/// orphan list, not a pointer into the code.
#[derive(Clone)]
pub struct RestoreNote {
    maps: MapService,
}

impl RestoreNote {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        path: &ReviewPath,
        old: LineRange,
        new: LineRange,
    ) -> Result<ReviewMap> {
        new.require_within(path)?;
        self.maps.edit(|map| {
            let orphan = map.take_orphan(slug, path.as_str(), old)?;
            map.add_line_note(slug, path.as_str(), new, orphan.text)
        })
    }
}

/// Let a deactivated note go, when the code it described is gone for good.
#[derive(Clone)]
pub struct DiscardNote {
    maps: MapService,
}

impl DiscardNote {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath, old: LineRange) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.take_orphan(slug, path.as_str(), old).map(|_| ()))
    }
}
