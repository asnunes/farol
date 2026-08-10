use crate::error::Result;
use crate::map::application::MapEditor;
use crate::map::domain::{Position, ReviewMap, Slug};

/// Move a block in the reading order.
#[derive(Clone)]
pub struct MoveBlock {
    maps: MapEditor,
}

impl MoveBlock {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug, position: Position) -> Result<ReviewMap> {
        self.maps.edit(|map| map.move_block(slug, position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::AddBlock;
    use crate::testing::{MapServices, slug, with_block};

    /// `core` first, `second` after it.
    fn two_blocks() -> MapServices {
        let (svc, _) = with_block(&["a.rs"]);
        AddBlock::new(svc.editor.clone())
            .execute(&slug("second"), "t", "c", Position::End, &[])
            .unwrap();
        svc
    }

    #[test]
    fn a_block_can_be_moved_ahead_of_another() {
        let svc = two_blocks();

        let map = MoveBlock::new(svc.editor)
            .execute(&slug("second"), Position::Before(slug("core")))
            .unwrap();

        assert_eq!(map.slugs(), vec!["second", "core"]);
    }

    #[test]
    fn a_block_can_be_moved_to_the_end() {
        let svc = two_blocks();

        let map = MoveBlock::new(svc.editor)
            .execute(&slug("core"), Position::End)
            .unwrap();

        assert_eq!(map.slugs(), vec!["second", "core"]);
    }

    #[test]
    fn a_block_can_be_moved_to_sit_after_another() {
        let svc = two_blocks();

        let map = MoveBlock::new(svc.editor)
            .execute(&slug("core"), Position::After(slug("second")))
            .unwrap();

        assert_eq!(map.slugs(), vec!["second", "core"]);
    }

    #[test]
    fn moving_a_block_where_it_already_is_changes_nothing() {
        let svc = two_blocks();

        let map = MoveBlock::new(svc.editor)
            .execute(&slug("second"), Position::After(slug("core")))
            .unwrap();

        assert_eq!(map.slugs(), vec!["core", "second"]);
    }

    #[test]
    fn moving_a_block_that_does_not_exist_is_refused() {
        let svc = two_blocks();

        assert!(
            MoveBlock::new(svc.editor)
                .execute(&slug("ghost"), Position::End)
                .is_err()
        );
    }

    #[test]
    fn moving_a_block_relative_to_one_that_does_not_exist_is_refused() {
        let svc = two_blocks();

        assert!(
            MoveBlock::new(svc.editor)
                .execute(&slug("core"), Position::Before(slug("nope")))
                .is_err()
        );
        assert_eq!(
            svc.versions.require_current().unwrap().slugs(),
            vec!["core", "second"],
            "a refused move must not have shuffled anything"
        );
    }
}
