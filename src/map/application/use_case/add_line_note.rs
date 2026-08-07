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
    fn a_note_lands_on_the_lines_it_was_written_for() {
        let (svc, scope) = with_block(&["a.rs"]);

        let map = AddLineNote::new(svc.editor)
            .execute(
                &slug("core"),
                &scope.path("a.rs").unwrap(),
                range(40, 52),
                "recovery only happens on a fresh transition".into(),
            )
            .unwrap();

        assert_eq!(
            line_notes(&map),
            vec![(
                range(40, 52),
                "recovery only happens on a fresh transition".to_string()
            )]
        );
    }

    #[test]
    fn several_notes_can_sit_on_one_file() {
        let (svc, scope) = with_block(&["a.rs"]);
        let path = scope.path("a.rs").unwrap();
        let add = AddLineNote::new(svc.editor);
        add.execute(&slug("core"), &path, range(10, 12), "first".into())
            .unwrap();

        let map = add
            .execute(&slug("core"), &path, range(40, 42), "second".into())
            .unwrap();

        assert_eq!(line_notes(&map).len(), 2);
    }

    #[test]
    fn writing_again_on_the_same_span_replaces_the_note() {
        // The span names the note, so a second write is a correction, not a
        // duplicate — which is what lets the skill be re-run over a map it
        // already half-wrote.
        let (svc, scope) = with_block(&["a.rs"]);
        let path = scope.path("a.rs").unwrap();
        let add = AddLineNote::new(svc.editor);
        add.execute(&slug("core"), &path, range(10, 12), "first".into())
            .unwrap();

        let map = add
            .execute(&slug("core"), &path, range(10, 12), "corrected".into())
            .unwrap();

        assert_eq!(
            line_notes(&map),
            vec![(range(10, 12), "corrected".to_string())]
        );
    }

    #[test]
    fn notes_come_back_in_the_order_they_will_be_read() {
        // Written out of order; the reader meets them going down the file.
        let (svc, scope) = with_block(&["a.rs"]);
        let path = scope.path("a.rs").unwrap();
        let add = AddLineNote::new(svc.editor);
        add.execute(&slug("core"), &path, range(80, 82), "later".into())
            .unwrap();

        let map = add
            .execute(&slug("core"), &path, range(10, 12), "earlier".into())
            .unwrap();

        let spans: Vec<_> = line_notes(&map).into_iter().map(|(r, _)| r).collect();
        assert_eq!(spans, vec![range(10, 12), range(80, 82)]);
    }

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
