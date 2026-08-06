use crate::map::application::MapEditor;
use crate::map::domain::{ReviewMap, Slug};
use crate::shared::error::Result;

/// Rewrite a block's title or the text that explains why it exists.
#[derive(Clone)]
pub struct UpdateBlock {
    maps: MapEditor,
}

impl UpdateBlock {
    pub fn new(maps: MapEditor) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{slug, with_block};

    #[test]
    fn rewriting_a_block_replaces_only_what_was_given() {
        let (svc, _) = with_block(&["a.rs"]);

        let map = UpdateBlock::new(svc.editor)
            .execute(&slug("core"), Some("A better title".into()), None)
            .unwrap();

        let block = map.block(&slug("core")).unwrap();
        assert_eq!(block.title, "A better title");
        assert_eq!(
            block.context, "c",
            "the prose was not what was being edited"
        );
    }

    #[test]
    fn editing_a_block_that_does_not_exist_lists_the_ones_that_do() {
        let (svc, _) = with_block(&["a.rs"]);

        let err = UpdateBlock::new(svc.editor)
            .execute(&slug("nope"), Some("t".into()), None)
            .unwrap_err();

        assert!(err.to_string().contains("core"), "{err}");
    }
}
