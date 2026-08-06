use crate::map::application::MapEditor;
use crate::map::domain::{ReviewMap, Slug};
use crate::shared::error::Result;

/// Drop a block. Its line notes survive as orphans, because the prose may still
/// be worth moving somewhere else.
#[derive(Clone)]
pub struct RemoveBlock {
    maps: MapEditor,
}

impl RemoveBlock {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(&self, slug: &Slug) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_block(slug))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::AddLineNote;
    use crate::testing::{range, slug, with_block};

    #[test]
    fn removing_a_block_keeps_its_notes_as_orphans_to_be_decided() {
        let (svc, scope) = with_block(&["a.rs"]);
        AddLineNote::new(svc.editor.clone())
            .execute(
                &slug("core"),
                &scope.path("a.rs").unwrap(),
                range(1, 2),
                "worth moving".into(),
            )
            .unwrap();

        let map = RemoveBlock::new(svc.editor).execute(&slug("core")).unwrap();

        assert!(map.block(&slug("core")).is_none());
        assert_eq!(map.orphans.len(), 1);
        assert_eq!(map.orphans[0].text, "worth moving");
    }
}
