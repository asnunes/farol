use crate::diff::domain::ReviewPath;
use crate::error::Result;
use crate::map::application::MapEditor;
use crate::map::domain::{LineRange, ReviewMap, Slug};

/// Let a deactivated note go, when the code it described is gone for good.
#[derive(Clone)]
pub struct DiscardNote {
    maps: MapEditor,
}

impl DiscardNote {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath, old: LineRange) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.take_orphan(slug, path.as_str(), old).map(|_| ()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{line_notes, orphaned, range, slug, with_block};

    #[test]
    fn discarding_drops_the_orphan_without_writing_a_note() {
        let (svc, scope) = with_block(&["a.rs"]);
        orphaned(&svc.editor, range(10, 12), "gone for good");

        let map = DiscardNote::new(svc.editor)
            .execute(&slug("core"), &scope.path("a.rs").unwrap(), range(10, 12))
            .unwrap();

        assert!(map.orphans().is_empty());
        assert!(line_notes(&map).is_empty());
    }

    #[test]
    fn discarding_something_that_was_never_orphaned_is_refused() {
        let (svc, scope) = with_block(&["a.rs"]);

        assert!(
            DiscardNote::new(svc.editor)
                .execute(&slug("core"), &scope.path("a.rs").unwrap(), range(10, 12))
                .is_err()
        );
    }
}
