use crate::diff::domain::ReviewPath;
use crate::map::application::MapEditor;
use crate::map::domain::{ReviewMap, Slug};
use crate::shared::error::Result;

/// Take a file out of a block — because the change to it was reverted, or it
/// turned out to belong elsewhere. Its line notes survive as orphans, the same
/// way removing the whole block leaves them.
#[derive(Clone)]
pub struct RemoveFile {
    maps: MapEditor,
}

impl RemoveFile {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_file(slug, path.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::AddLineNote;
    use crate::testing::{block_paths, range, slug, use_case_setup, with_block};

    #[test]
    fn removing_a_file_leaves_the_others_in_order() {
        let (svc, scope) = with_block(&["a.rs", "b.rs"]);

        let map = RemoveFile::new(svc.editor)
            .execute(&slug("core"), &scope.path("a.rs").unwrap())
            .unwrap();

        assert_eq!(block_paths(&map), vec!["b.rs"]);
    }

    #[test]
    fn removing_a_file_keeps_its_notes_as_orphans() {
        // The prose cost something to write. Dropping it silently because the
        // file moved to another block would be the expensive kind of loss.
        let (svc, scope) = with_block(&["a.rs"]);
        let a = scope.path("a.rs").unwrap();
        AddLineNote::new(svc.editor.clone())
            .execute(&slug("core"), &a, range(3, 4), "worth moving".into())
            .unwrap();

        let map = RemoveFile::new(svc.editor)
            .execute(&slug("core"), &a)
            .unwrap();

        assert_eq!(map.orphans.len(), 1);
        assert_eq!(map.orphans[0].text, "worth moving");
    }

    #[test]
    fn a_file_that_is_not_in_the_block_cannot_be_removed_from_it() {
        let (svc, _) = with_block(&["a.rs"]);
        let (_, wider) = use_case_setup(&["a.rs", "b.rs"]);

        let err = RemoveFile::new(svc.editor)
            .execute(&slug("core"), &wider.path("b.rs").unwrap())
            .unwrap_err();

        assert!(err.to_string().contains("b.rs"), "{err}");
    }
}
