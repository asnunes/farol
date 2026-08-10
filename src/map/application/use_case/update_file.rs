use crate::diff::domain::ReviewPath;
use crate::error::Result;
use crate::map::application::MapEditor;
use crate::map::domain::{ReviewMap, Slug};

/// Rewrite the note that explains what a file contributes to its block.
#[derive(Clone)]
pub struct UpdateFile {
    maps: MapEditor,
}

impl UpdateFile {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath, note: String) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.update_file(slug, path.as_str(), Some(note)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{slug, with_block};

    #[test]
    fn rewriting_a_file_note_replaces_the_prose() {
        let (svc, scope) = with_block(&["a.rs"]);
        let a = scope.path("a.rs").unwrap();
        let update = UpdateFile::new(svc.editor);
        update
            .execute(&slug("core"), &a, "first thought".into())
            .unwrap();

        let map = update
            .execute(&slug("core"), &a, "what I actually meant".into())
            .unwrap();

        let file = map.block(&slug("core")).unwrap().file("a.rs").unwrap();
        assert_eq!(file.note.as_deref(), Some("what I actually meant"));
    }

    #[test]
    fn a_note_cannot_be_written_on_a_file_the_block_does_not_hold() {
        let (svc, scope) = with_block(&["a.rs", "b.rs"]);
        crate::map::application::RemoveFile::new(svc.editor.clone())
            .execute(&slug("core"), &scope.path("b.rs").unwrap())
            .unwrap();

        assert!(
            UpdateFile::new(svc.editor)
                .execute(&slug("core"), &scope.path("b.rs").unwrap(), "stray".into())
                .is_err()
        );
    }
}
