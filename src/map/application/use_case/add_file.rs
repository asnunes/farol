use crate::diff::domain::ReviewPath;
use crate::map::application::MapEditor;
use crate::map::domain::{ReviewMap, Slug};
use crate::shared::error::Result;

/// Put a file into a block, at a chosen place in its reading order.
#[derive(Clone)]
pub struct AddFile {
    maps: MapEditor,
}

impl AddFile {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        path: &ReviewPath,
        note: Option<String>,
        after: Option<&ReviewPath>,
    ) -> Result<ReviewMap> {
        let after = after.map(ReviewPath::as_str);
        self.maps
            .edit(|map| map.add_file(slug, path.as_str(), note, after))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{block_paths, slug, use_case_setup, with_block};

    #[test]
    fn a_file_lands_after_the_one_it_was_placed_behind() {
        // The reading order inside a block is the whole point of the block, so
        // placement is not a detail the caller can be made to redo by hand.
        let (svc, scope) = with_block(&["a.rs", "b.rs"]);
        let a = scope.path("a.rs").unwrap();
        let (_, wider) = use_case_setup(&["a.rs", "b.rs", "c.rs"]);

        let map = AddFile::new(svc.editor)
            .execute(&slug("core"), &wider.path("c.rs").unwrap(), None, Some(&a))
            .unwrap();

        assert_eq!(block_paths(&map), vec!["a.rs", "c.rs", "b.rs"]);
    }

    #[test]
    fn a_file_with_no_placement_goes_to_the_end() {
        let (svc, _) = with_block(&["a.rs"]);
        let (_, wider) = use_case_setup(&["a.rs", "b.rs"]);

        let map = AddFile::new(svc.editor)
            .execute(&slug("core"), &wider.path("b.rs").unwrap(), None, None)
            .unwrap();

        assert_eq!(block_paths(&map), vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn a_file_can_arrive_with_the_note_that_explains_it() {
        // Adding and explaining in one step is what the skill does; making it
        // two calls would leave a window where the file has no reason to be
        // there.
        let (svc, _) = with_block(&["a.rs"]);
        let (_, wider) = use_case_setup(&["a.rs", "b.rs"]);

        let map = AddFile::new(svc.editor)
            .execute(
                &slug("core"),
                &wider.path("b.rs").unwrap(),
                Some("the deletions here are not an additional change".into()),
                None,
            )
            .unwrap();

        let file = map.block(&slug("core")).unwrap().file("b.rs").unwrap();
        assert_eq!(
            file.note.as_deref(),
            Some("the deletions here are not an additional change")
        );
    }

    #[test]
    fn a_file_added_without_a_note_simply_has_none() {
        // Most files need no prose of their own; the block's context covers
        // them, and an empty note would be noise in the header.
        let (svc, _) = with_block(&["a.rs"]);

        let map = svc.versions.require_current().unwrap();

        let file = map.block(&slug("core")).unwrap().file("a.rs").unwrap();
        assert_eq!(file.note, None);
    }

    #[test]
    fn the_same_file_cannot_be_listed_twice_in_one_block() {
        let (svc, scope) = with_block(&["a.rs"]);

        assert!(
            AddFile::new(svc.editor)
                .execute(&slug("core"), &scope.path("a.rs").unwrap(), None, None)
                .is_err()
        );
    }

    #[test]
    fn a_file_cannot_be_added_to_a_block_that_does_not_exist() {
        let (svc, scope) = use_case_setup(&["a.rs"]);

        let err = AddFile::new(svc.editor)
            .execute(&slug("nope"), &scope.path("a.rs").unwrap(), None, None)
            .unwrap_err();

        assert!(err.to_string().contains("nope"), "{err}");
    }
}
