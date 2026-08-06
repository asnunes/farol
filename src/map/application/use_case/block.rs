use crate::diff::domain::ReviewPath;
use crate::map::application::MapService;
use crate::map::domain::{Position, ReviewMap, Slug};
use crate::shared::error::Result;

/// Open a block and, optionally, seed it with files that need no note.
#[derive(Clone)]
pub struct AddBlock {
    maps: MapService,
}

impl AddBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        title: &str,
        context: &str,
        position: Position,
        paths: &[ReviewPath],
    ) -> Result<ReviewMap> {
        self.maps.edit(|map| {
            map.add_block(slug, title, context, position)?;
            for path in paths {
                map.add_file(slug, path.as_str(), None, None)?;
            }
            Ok(())
        })
    }
}

/// Rewrite a block's title or the text that explains why it exists.
#[derive(Clone)]
pub struct UpdateBlock {
    maps: MapService,
}

impl UpdateBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(
        &self,
        slug: &Slug,
        title: Option<String>,
        context: Option<String>,
    ) -> Result<ReviewMap> {
        self.maps.edit(|map| map.update_block(slug, title, context))
    }
}

/// Drop a block. Its line notes survive as orphans, because the prose may still
/// be worth moving somewhere else.
#[derive(Clone)]
pub struct RemoveBlock {
    maps: MapService,
}

impl RemoveBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_block(slug))
    }
}

/// Move a block in the reading order.
#[derive(Clone)]
pub struct MoveBlock {
    maps: MapService,
}

impl MoveBlock {
    pub fn new(maps: MapService) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, position: Position) -> Result<ReviewMap> {
        self.maps.edit(|map| map.move_block(slug, position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::application::ReviewScope;
    use crate::map::application::AddLineNote;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, service, slug};
    use std::sync::Arc;

    fn setup(paths: &[&str]) -> (MapService, ReviewScope) {
        let source = Arc::new(FakeDiffSource::with_paths(paths));
        let maps = service(
            FakeDiffSource::with_paths(paths),
            Arc::new(InMemoryMapRepository::new()),
        );
        (maps, ReviewScope::new(source))
    }

    #[test]
    fn a_block_is_opened_with_its_files_in_the_order_they_were_given() {
        let (maps, scope) = setup(&["a.rs", "b.rs"]);
        let files = scope.paths(&["b.rs".into(), "a.rs".into()]).unwrap();

        let map = AddBlock::new(maps)
            .execute(&slug("core"), "The change", "why", Position::End, &files)
            .unwrap();

        let paths: Vec<&str> = map
            .block(&slug("core"))
            .unwrap()
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec!["b.rs", "a.rs"],
            "the caller's order is the reading order"
        );
    }

    #[test]
    fn a_block_that_fails_partway_leaves_nothing_behind() {
        // The second file duplicates the first, so the whole thing must fail
        // rather than leave a half-populated block on screen.
        let (maps, scope) = setup(&["a.rs"]);
        let same = scope.paths(&["a.rs".into(), "a.rs".into()]).unwrap();

        assert!(
            AddBlock::new(maps.clone())
                .execute(&slug("core"), "t", "c", Position::End, &same)
                .is_err()
        );
        assert!(
            maps.require_current()
                .unwrap()
                .block(&slug("core"))
                .is_none()
        );
    }

    #[test]
    fn a_new_block_can_be_placed_ahead_of_an_existing_one() {
        let (maps, _) = setup(&["a.rs"]);
        AddBlock::new(maps.clone())
            .execute(&slug("alert"), "The alert", "c", Position::End, &[])
            .unwrap();

        let map = AddBlock::new(maps)
            .execute(
                &slug("metric"),
                "The metric",
                "c",
                Position::Before(slug("alert")),
                &[],
            )
            .unwrap();

        assert_eq!(map.slugs(), vec!["metric", "alert"]);
    }

    #[test]
    fn removing_a_block_keeps_its_notes_as_orphans_to_be_decided() {
        let (maps, scope) = setup(&["a.rs"]);
        let files = scope.paths(&["a.rs".into()]).unwrap();
        AddBlock::new(maps.clone())
            .execute(&slug("core"), "t", "c", Position::End, &files)
            .unwrap();
        AddLineNote::new(maps.clone())
            .execute(
                &slug("core"),
                &files[0],
                crate::map::domain::LineRange::new(1, 2).unwrap(),
                "worth moving".into(),
            )
            .unwrap();

        let map = RemoveBlock::new(maps).execute(&slug("core")).unwrap();

        assert!(map.block(&slug("core")).is_none());
        assert_eq!(map.orphans.len(), 1);
        assert_eq!(map.orphans[0].text, "worth moving");
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;
    use crate::testing::{slug, use_case_setup};

    fn with_two_blocks() -> MapService {
        let (maps, _) = use_case_setup(&["a.rs"]);
        for name in ["first", "second"] {
            AddBlock::new(maps.clone())
                .execute(&slug(name), "t", "c", Position::End, &[])
                .unwrap();
        }
        maps
    }

    #[test]
    fn rewriting_a_block_replaces_only_what_was_given() {
        let maps = with_two_blocks();

        let map = UpdateBlock::new(maps)
            .execute(&slug("first"), Some("A better title".into()), None)
            .unwrap();

        let block = map.block(&slug("first")).unwrap();
        assert_eq!(block.title, "A better title");
        assert_eq!(
            block.context, "c",
            "the prose was not what was being edited"
        );
    }

    #[test]
    fn a_block_can_be_moved_ahead_of_another() {
        let maps = with_two_blocks();

        let map = MoveBlock::new(maps)
            .execute(&slug("second"), Position::Before(slug("first")))
            .unwrap();

        assert_eq!(map.slugs(), vec!["second", "first"]);
    }

    #[test]
    fn a_block_can_be_moved_to_the_end() {
        let maps = with_two_blocks();

        let map = MoveBlock::new(maps)
            .execute(&slug("first"), Position::End)
            .unwrap();

        assert_eq!(map.slugs(), vec!["second", "first"]);
    }

    #[test]
    fn moving_a_block_relative_to_one_that_does_not_exist_is_refused() {
        let maps = with_two_blocks();

        assert!(
            MoveBlock::new(maps.clone())
                .execute(&slug("first"), Position::Before(slug("nope")))
                .is_err()
        );
        assert_eq!(
            maps.require_current().unwrap().slugs(),
            vec!["first", "second"],
            "a refused move must not have shuffled anything"
        );
    }

    #[test]
    fn editing_a_block_that_does_not_exist_lists_the_ones_that_do() {
        let maps = with_two_blocks();

        let err = UpdateBlock::new(maps)
            .execute(&slug("nope"), Some("t".into()), None)
            .unwrap_err();

        assert!(err.to_string().contains("first"), "{err}");
    }
}
