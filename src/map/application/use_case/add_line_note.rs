use crate::diff::domain::ReviewPath;
use crate::map::application::MapEditor;
use crate::map::domain::{LineRange, ReviewMap, Slug};
use crate::shared::error::Result;

/// Pin a note to a span of lines. The span is checked against the file here,
/// because a note past the end would render nowhere.
#[derive(Clone)]
pub struct AddLineNote {
    maps: MapEditor,
}

impl AddLineNote {
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
        range.require_within(path)?;
        self.maps
            .edit(|map| map.add_line_note(slug, path.as_str(), range, note))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{line_notes, range, slug, with_block};

    #[test]
    fn a_note_pointing_past_the_end_of_the_file_is_refused() {
        // The fake serves files of a hundred lines.
        let (svc, scope) = with_block(&["a.rs"]);

        let err = AddLineNote::new(svc.editor)
            .execute(
                &slug("core"),
                &scope.path("a.rs").unwrap(),
                range(200, 210),
                "nowhere".into(),
            )
            .unwrap_err();

        assert!(
            err.to_string().contains("100 lines"),
            "the error should say how long the file actually is: {err}"
        );
        assert!(
            line_notes(&svc.versions.require_current().unwrap()).is_empty(),
            "a refused note must not be half-written"
        );
    }

    #[test]
    fn a_note_ending_exactly_on_the_last_line_is_allowed() {
        let (svc, scope) = with_block(&["a.rs"]);

        AddLineNote::new(svc.editor)
            .execute(
                &slug("core"),
                &scope.path("a.rs").unwrap(),
                range(90, 100),
                "the tail".into(),
            )
            .unwrap();
    }
}
