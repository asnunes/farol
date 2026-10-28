use crate::diff::domain::ReviewPath;
use crate::error::Result;
use crate::map::application::MapEditor;
use crate::map::domain::{ReviewMap, Slug};

/// Say why a file is worth a closer look, or take that back.
///
/// The inverse of what skim used to say: with lenses, every file starts
/// folded, and this is the one mark that asks for the code to be opened. The
/// reason is what makes it worth trusting — important, dense, or wrong in a
/// way the reader should see for themselves.
#[derive(Clone)]
pub struct LookFile {
    maps: MapEditor,
}

impl LookFile {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        path: &ReviewPath,
        reason: Option<String>,
    ) -> Result<ReviewMap> {
        self.maps.edit(|map| map.look(slug, path.as_str(), reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{slug, with_block};

    #[test]
    fn a_file_marked_worth_a_closer_look_carries_why() {
        let (svc, scope) = with_block(&["a.rs"]);

        let map = LookFile::new(svc.editor)
            .execute(
                &slug("core"),
                &scope.path("a.rs").unwrap(),
                Some("the whole rule lives here".into()),
            )
            .unwrap();

        let file = map.block(&slug("core")).unwrap().file("a.rs").unwrap();
        assert_eq!(file.look.as_deref(), Some("the whole rule lives here"));
    }
}
