use crate::diff::domain::ReviewPath;
use crate::map::application::MapEditor;
use crate::map::domain::{Position, ReviewMap, Slug};
use crate::shared::error::Result;

/// Open a block and, optionally, seed it with files that need no note.
#[derive(Clone)]
pub struct AddBlock {
    maps: MapEditor,
}

impl AddBlock {
    pub fn new(maps: MapEditor) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{block_paths, slug, use_case_setup};

    #[test]
    fn a_block_is_opened_with_its_files_in_the_order_they_were_given() {
        let (svc, scope) = use_case_setup(&["a.rs", "b.rs"]);
        let files = scope.paths(&["b.rs".into(), "a.rs".into()]).unwrap();

        let map = AddBlock::new(svc.editor)
            .execute(&slug("core"), "The change", "why", Position::End, &files)
            .unwrap();

        assert_eq!(
            block_paths(&map),
            vec!["b.rs", "a.rs"],
            "the caller's order is the reading order"
        );
    }

    #[test]
    fn a_block_that_fails_partway_leaves_nothing_behind() {
        // The second file duplicates the first, so the whole thing must fail
        // rather than leave a half-populated block on screen.
        let (svc, scope) = use_case_setup(&["a.rs"]);
        let same = scope.paths(&["a.rs".into(), "a.rs".into()]).unwrap();

        assert!(
            AddBlock::new(svc.editor)
                .execute(&slug("core"), "t", "c", Position::End, &same)
                .is_err()
        );
        assert!(
            svc.versions
                .require_current()
                .unwrap()
                .block(&slug("core"))
                .is_none()
        );
    }

    #[test]
    fn a_new_block_can_be_placed_ahead_of_an_existing_one() {
        let (svc, _) = use_case_setup(&["a.rs"]);
        let add = AddBlock::new(svc.editor);
        add.execute(&slug("alert"), "The alert", "c", Position::End, &[])
            .unwrap();

        let map = add
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
}
