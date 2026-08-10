use crate::diff::domain::ReviewPath;
use crate::error::Result;
use crate::map::application::MapEditor;
use crate::map::domain::{LineRange, ReviewMap, Slug};

/// Withdraw a note on purpose. Unlike a note the code moved out from under,
/// this one is not kept as an orphan: there is nothing to restore later.
#[derive(Clone)]
pub struct RemoveLineNote {
    maps: MapEditor,
}

impl RemoveLineNote {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath, range: LineRange) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.remove_line_note(slug, path.as_str(), range))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::AddLineNote;
    use crate::testing::{line_notes, range, slug, with_block};

    fn with_a_note() -> (crate::testing::MapServices, ReviewPath) {
        let (svc, scope) = with_block(&["a.rs"]);
        let path = scope.path("a.rs").unwrap();
        AddLineNote::new(svc.editor.clone())
            .execute(&slug("core"), &path, range(10, 12), "first thought".into())
            .unwrap();
        (svc, path)
    }

    #[test]
    fn removing_a_note_takes_it_away_for_good() {
        let (svc, path) = with_a_note();

        let map = RemoveLineNote::new(svc.editor)
            .execute(&slug("core"), &path, range(10, 12))
            .unwrap();

        assert!(line_notes(&map).is_empty());
        assert!(map.orphans().is_empty());
    }

    #[test]
    fn removing_a_note_that_is_not_there_is_refused() {
        let (svc, path) = with_a_note();

        assert!(
            RemoveLineNote::new(svc.editor)
                .execute(&slug("core"), &path, range(30, 31))
                .is_err()
        );
    }

    #[test]
    fn a_note_is_identified_by_its_exact_span() {
        // An overlapping range is a different note, not the same one.
        let (svc, path) = with_a_note();

        assert!(
            RemoveLineNote::new(svc.editor)
                .execute(&slug("core"), &path, range(10, 11))
                .is_err()
        );
    }
}
