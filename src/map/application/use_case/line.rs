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
    use crate::map::domain::{Orphan, OrphanReason, Position};
    use crate::testing::{slug, use_case_setup};

    fn range(from: u32, to: u32) -> LineRange {
        LineRange::new(from, to).unwrap()
    }

    /// A block with one file, on a fake where the file is 100 lines long.
    fn with_block() -> (crate::testing::MapServices, ReviewPath) {
        let (svc, scope) = use_case_setup(&["a.rs"]);
        let maps = svc.editor.clone();
        let path = scope.path("a.rs").unwrap();
        crate::map::application::AddBlock::new(maps.clone())
            .execute(
                &slug("core"),
                "t",
                "c",
                Position::End,
                std::slice::from_ref(&path),
            )
            .unwrap();
        (svc, path)
    }

    /// The state a derivation leaves behind when a note's code was rewritten.
    fn orphaned(
        maps: &crate::map::application::MapEditor,
        old: LineRange,
        text: &str,
    ) -> Result<()> {
        maps.edit(|map| {
            map.orphans.push(Orphan {
                block: slug("core"),
                path: "a.rs".into(),
                old_range: old,
                snapshot: "the code it covered".into(),
                reason: OrphanReason::HunkOverlap,
                text: text.into(),
            });
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn a_note_pointing_past_the_end_of_the_file_is_refused() {
        let (svc, path) = with_block();
        let maps = svc.editor.clone();

        let err = AddLineNote::new(maps.clone())
            .execute(&slug("core"), &path, range(200, 210), "nowhere".into())
            .unwrap_err();

        assert!(
            err.to_string().contains("100 lines"),
            "the error should say how long the file actually is: {err}"
        );
        assert!(
            svc.versions.require_current().unwrap().orphans.is_empty()
                && svc.versions.require_current().unwrap().blocks[0].files[0]
                    .line_notes
                    .is_empty(),
            "a refused note must not be half-written"
        );
    }

    #[test]
    fn a_note_ending_exactly_on_the_last_line_is_allowed() {
        let (svc, path) = with_block();
        let maps = svc.editor.clone();

        AddLineNote::new(maps)
            .execute(&slug("core"), &path, range(90, 100), "the tail".into())
            .unwrap();
    }

    #[test]
    fn restoring_moves_the_prose_to_the_new_span_and_clears_the_orphan() {
        let (svc, path) = with_block();
        let maps = svc.editor.clone();
        orphaned(&maps, range(10, 12), "expensive prose").unwrap();

        let map = RestoreNote::new(maps)
            .execute(&slug("core"), &path, range(10, 12), range(40, 42))
            .unwrap();

        let notes = &map.block(&slug("core")).unwrap().files[0].line_notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].range, range(40, 42));
        assert_eq!(
            notes[0].text, "expensive prose",
            "the prose is the point of restoring"
        );
        assert!(map.orphans.is_empty());
    }

    #[test]
    fn restoring_refuses_a_new_span_past_the_end_of_the_file() {
        let (svc, path) = with_block();
        let maps = svc.editor.clone();
        orphaned(&maps, range(95, 99), "n").unwrap();

        assert!(
            RestoreNote::new(maps.clone())
                .execute(&slug("core"), &path, range(95, 99), range(300, 301))
                .is_err()
        );
        assert_eq!(
            svc.versions.require_current().unwrap().orphans.len(),
            1,
            "and the orphan survives to be tried again"
        );
    }

    #[test]
    fn restoring_does_not_check_the_old_span_against_the_file() {
        // The old range is a key into the orphan list, not a pointer into the
        // code: the file has since shrunk past it, and that is not a reason to
        // refuse a restore to a span that does exist.
        let (svc, path) = with_block();
        let maps = svc.editor.clone();
        orphaned(&maps, range(150, 160), "written when the file was longer").unwrap();

        let map = RestoreNote::new(maps)
            .execute(&slug("core"), &path, range(150, 160), range(40, 42))
            .unwrap();

        assert_eq!(
            map.block(&slug("core")).unwrap().files[0].line_notes[0].range,
            range(40, 42)
        );
    }

    #[test]
    fn discarding_drops_the_orphan_without_writing_a_note() {
        let (svc, path) = with_block();
        let maps = svc.editor.clone();
        orphaned(&maps, range(10, 12), "gone for good").unwrap();

        let map = DiscardNote::new(maps)
            .execute(&slug("core"), &path, range(10, 12))
            .unwrap();

        assert!(map.orphans.is_empty());
        assert!(
            map.block(&slug("core")).unwrap().files[0]
                .line_notes
                .is_empty()
        );
    }

    #[test]
    fn an_orphan_that_was_never_recorded_cannot_be_restored() {
        let (svc, path) = with_block();
        let maps = svc.editor.clone();

        assert!(
            RestoreNote::new(maps)
                .execute(&slug("core"), &path, range(10, 12), range(40, 42))
                .is_err()
        );
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;
    use crate::map::application::AddBlock;
    use crate::map::domain::Position;
    use crate::testing::{slug, use_case_setup};

    fn range(from: u32, to: u32) -> LineRange {
        LineRange::new(from, to).unwrap()
    }

    fn with_a_note() -> (crate::map::application::MapEditor, ReviewPath) {
        let (svc, scope) = use_case_setup(&["a.rs"]);
        let maps = svc.editor.clone();
        let path = scope.path("a.rs").unwrap();
        AddBlock::new(maps.clone())
            .execute(
                &slug("core"),
                "t",
                "c",
                Position::End,
                std::slice::from_ref(&path),
            )
            .unwrap();
        AddLineNote::new(maps.clone())
            .execute(&slug("core"), &path, range(10, 12), "first thought".into())
            .unwrap();
        (maps, path)
    }

    fn notes(map: &ReviewMap) -> Vec<(LineRange, String)> {
        map.block(&slug("core")).unwrap().files[0]
            .line_notes
            .iter()
            .map(|n| (n.range, n.text.clone()))
            .collect()
    }

    #[test]
    fn rewriting_a_note_keeps_it_on_the_same_lines() {
        let (maps, path) = with_a_note();

        let map = UpdateLineNote::new(maps)
            .execute(
                &slug("core"),
                &path,
                range(10, 12),
                "what I actually meant".into(),
            )
            .unwrap();

        assert_eq!(
            notes(&map),
            vec![(range(10, 12), "what I actually meant".to_string())]
        );
    }

    #[test]
    fn rewriting_a_note_that_is_not_there_is_refused() {
        // Silently creating one would put prose on lines nobody chose.
        let (maps, path) = with_a_note();

        assert!(
            UpdateLineNote::new(maps)
                .execute(&slug("core"), &path, range(30, 31), "stray".into())
                .is_err()
        );
    }

    #[test]
    fn removing_a_note_takes_it_away_for_good() {
        // Unlike a note the code moved out from under, this one is being
        // withdrawn on purpose: there is nothing to restore later.
        let (maps, path) = with_a_note();

        let map = RemoveLineNote::new(maps)
            .execute(&slug("core"), &path, range(10, 12))
            .unwrap();

        assert!(notes(&map).is_empty());
        assert!(map.orphans.is_empty());
    }

    #[test]
    fn removing_a_note_that_is_not_there_is_refused() {
        let (maps, path) = with_a_note();

        assert!(
            RemoveLineNote::new(maps)
                .execute(&slug("core"), &path, range(30, 31))
                .is_err()
        );
    }

    #[test]
    fn a_note_is_identified_by_its_exact_span() {
        // An overlapping range is a different note, not the same one.
        let (maps, path) = with_a_note();

        assert!(
            RemoveLineNote::new(maps)
                .execute(&slug("core"), &path, range(10, 11))
                .is_err()
        );
    }
}
