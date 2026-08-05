use crate::diff::domain::ReviewPath;
use crate::map::application::MapService;
use crate::map::domain::{ReviewMap, Slug};
use crate::shared::error::Result;

/// Put a file into a block, at a chosen place in its reading order.
#[derive(Clone)]
pub struct AddFile {
    maps: MapService,
}

impl AddFile {
    pub fn new(maps: MapService) -> Self {
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

/// Rewrite the note that explains what a file contributes to its block.
#[derive(Clone)]
pub struct UpdateFile {
    maps: MapService,
}

impl UpdateFile {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath, note: String) -> Result<ReviewMap> {
        self.maps
            .edit(|map| map.update_file(slug, path.as_str(), Some(note)))
    }
}

/// Take a file out of a block — because the change to it was reverted, or it
/// turned out to belong elsewhere.
#[derive(Clone)]
pub struct RemoveFile {
    maps: MapService,
}

impl RemoveFile {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, path: &ReviewPath) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_file(slug, path.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::AddBlock;
    use crate::map::domain::Position;
    use crate::testing::{slug, use_case_setup};

    fn paths_of(map: &ReviewMap) -> Vec<String> {
        map.block(&slug("core"))
            .unwrap()
            .files
            .iter()
            .map(|f| f.path.clone())
            .collect()
    }

    #[test]
    fn a_file_lands_after_the_one_it_was_placed_behind() {
        // The reading order inside a block is the whole point of the block, so
        // placement is not a detail the caller can be made to redo by hand.
        let (maps, scope) = use_case_setup(&["a.rs", "b.rs", "c.rs"]);
        let a = scope.path("a.rs").unwrap();
        let b = scope.path("b.rs").unwrap();
        AddBlock::new(maps.clone())
            .execute(&slug("core"), "t", "c", Position::End, &[a.clone(), b])
            .unwrap();

        let map = AddFile::new(maps)
            .execute(&slug("core"), &scope.path("c.rs").unwrap(), None, Some(&a))
            .unwrap();

        assert_eq!(paths_of(&map), vec!["a.rs", "c.rs", "b.rs"]);
    }

    #[test]
    fn a_file_with_no_placement_goes_to_the_end() {
        let (maps, scope) = use_case_setup(&["a.rs", "b.rs"]);
        AddBlock::new(maps.clone())
            .execute(
                &slug("core"),
                "t",
                "c",
                Position::End,
                &[scope.path("a.rs").unwrap()],
            )
            .unwrap();

        let map = AddFile::new(maps)
            .execute(&slug("core"), &scope.path("b.rs").unwrap(), None, None)
            .unwrap();

        assert_eq!(paths_of(&map), vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn the_same_file_cannot_be_listed_twice_in_one_block() {
        let (maps, scope) = use_case_setup(&["a.rs"]);
        let a = scope.path("a.rs").unwrap();
        AddBlock::new(maps.clone())
            .execute(
                &slug("core"),
                "t",
                "c",
                Position::End,
                std::slice::from_ref(&a),
            )
            .unwrap();

        assert!(
            AddFile::new(maps)
                .execute(&slug("core"), &a, None, None)
                .is_err()
        );
    }

    #[test]
    fn a_file_cannot_be_added_to_a_block_that_does_not_exist() {
        let (maps, scope) = use_case_setup(&["a.rs"]);

        let err = AddFile::new(maps)
            .execute(&slug("nope"), &scope.path("a.rs").unwrap(), None, None)
            .unwrap_err();

        assert!(err.to_string().contains("nope"), "{err}");
    }
}
