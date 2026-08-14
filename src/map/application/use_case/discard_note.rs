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

    /// The path is a key into the map, not a file to be found.
    ///
    /// Deliberately a plain string: what is being discarded is a note about
    /// code that is gone, and the commonest way for code to be gone is for the
    /// file to have been renamed or deleted. Proving the path against the
    /// review first would refuse exactly the orphans that most need settling,
    /// and `map check` would stay red with no way to clear it.
    pub fn execute(&self, slug: &Slug, path: &str, old: LineRange) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.take_orphan(slug, path, old).map(|_| ()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{line_notes, orphaned, range, slug, with_block};

    #[test]
    fn discarding_drops_the_orphan_without_writing_a_note() {
        let (svc, _scope) = with_block(&["a.rs"]);
        orphaned(&svc.editor, range(10, 12), "gone for good");

        let map = DiscardNote::new(svc.editor)
            .execute(&slug("core"), "a.rs", range(10, 12))
            .unwrap();

        assert!(map.orphans().is_empty());
        assert!(line_notes(&map).is_empty());
    }

    #[test]
    fn discarding_something_that_was_never_orphaned_is_refused() {
        let (svc, _scope) = with_block(&["a.rs"]);

        assert!(
            DiscardNote::new(svc.editor)
                .execute(&slug("core"), "a.rs", range(10, 12))
                .is_err()
        );
    }
}
