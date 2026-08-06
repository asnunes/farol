use crate::diff::domain::ReviewPath;
use crate::map::application::MapEditor;
use crate::map::domain::{LineRange, ReviewMap, Slug};
use crate::shared::error::Result;

/// Rewrite a note whose span did not move.
#[derive(Clone)]
pub struct UpdateLineNote {
    maps: MapEditor,
}

impl UpdateLineNote {
    pub fn new(maps: MapEditor) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::AddLineNote;
    use crate::testing::{line_notes, range, slug, with_block};

    #[test]
    fn rewriting_a_note_keeps_it_on_the_same_lines() {
        let (svc, scope) = with_block(&["a.rs"]);
        let a = scope.path("a.rs").unwrap();
        AddLineNote::new(svc.editor.clone())
            .execute(&slug("core"), &a, range(10, 12), "first thought".into())
            .unwrap();

        let map = UpdateLineNote::new(svc.editor)
            .execute(
                &slug("core"),
                &a,
                range(10, 12),
                "what I actually meant".into(),
            )
            .unwrap();

        assert_eq!(
            line_notes(&map),
            vec![(range(10, 12), "what I actually meant".to_string())]
        );
    }

    #[test]
    fn rewriting_a_note_that_is_not_there_is_refused() {
        // Silently creating one would put prose on lines nobody chose.
        let (svc, scope) = with_block(&["a.rs"]);

        assert!(
            UpdateLineNote::new(svc.editor)
                .execute(
                    &slug("core"),
                    &scope.path("a.rs").unwrap(),
                    range(30, 31),
                    "stray".into(),
                )
                .is_err()
        );
    }
}
