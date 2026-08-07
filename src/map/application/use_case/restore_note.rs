use crate::diff::domain::ReviewPath;
use crate::map::application::MapEditor;
use crate::map::domain::{LineRange, ReviewMap, Slug};
use crate::shared::error::Result;

/// Bring a deactivated note back where its code moved to.
///
/// Only the new span is checked against the file; the old one is a key into the
/// orphan list, not a pointer into the code.
#[derive(Clone)]
pub struct RestoreNote {
    maps: MapEditor,
}

impl RestoreNote {
    pub fn new(maps: MapEditor) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{line_notes, orphaned, range, slug, with_block};

    #[test]
    fn restoring_moves_the_prose_to_the_new_span_and_clears_the_orphan() {
        let (svc, scope) = with_block(&["a.rs"]);
        let path = scope.path("a.rs").unwrap();
        orphaned(&svc.editor, range(10, 12), "expensive prose");

        let map = RestoreNote::new(svc.editor)
            .execute(&slug("core"), &path, range(10, 12), range(40, 42))
            .unwrap();

        assert_eq!(
            line_notes(&map),
            vec![(range(40, 42), "expensive prose".to_string())],
            "the prose is the point of restoring"
        );
        assert!(map.orphans().is_empty());
    }

    #[test]
    fn restoring_refuses_a_new_span_past_the_end_of_the_file() {
        let (svc, scope) = with_block(&["a.rs"]);
        let path = scope.path("a.rs").unwrap();
        orphaned(&svc.editor, range(95, 99), "n");

        assert!(
            RestoreNote::new(svc.editor.clone())
                .execute(&slug("core"), &path, range(95, 99), range(300, 301))
                .is_err()
        );
        assert_eq!(
            svc.versions.require_current().unwrap().orphans().len(),
            1,
            "and the orphan survives to be tried again"
        );
    }

    #[test]
    fn restoring_does_not_check_the_old_span_against_the_file() {
        // The old range is a key into the orphan list, not a pointer into the
        // code: the file has since shrunk past it, and that is not a reason to
        // refuse a restore to a span that does exist.
        let (svc, scope) = with_block(&["a.rs"]);
        let path = scope.path("a.rs").unwrap();
        orphaned(
            &svc.editor,
            range(150, 160),
            "written when the file was longer",
        );

        let map = RestoreNote::new(svc.editor)
            .execute(&slug("core"), &path, range(150, 160), range(40, 42))
            .unwrap();

        assert_eq!(line_notes(&map)[0].0, range(40, 42));
    }

    #[test]
    fn an_orphan_that_was_never_recorded_cannot_be_restored() {
        let (svc, scope) = with_block(&["a.rs"]);

        assert!(
            RestoreNote::new(svc.editor)
                .execute(
                    &slug("core"),
                    &scope.path("a.rs").unwrap(),
                    range(10, 12),
                    range(40, 42),
                )
                .is_err()
        );
    }
}
