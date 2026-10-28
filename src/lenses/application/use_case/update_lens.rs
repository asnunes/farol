use std::sync::Arc;

use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensCatalogRepository, LensId};

/// Change a lens's title, prompt or switch, in one write: the panel sends
/// them together, and a switch that failed must not leave the words saved.
#[derive(Clone)]
pub struct UpdateLens {
    catalog: Arc<dyn LensCatalogRepository>,
}

impl UpdateLens {
    pub fn new(catalog: Arc<dyn LensCatalogRepository>) -> Self {
        Self { catalog }
    }

    pub fn execute(
        &self,
        id: &LensId,
        title: Option<&str>,
        prompt: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<LensCatalog> {
        let mut catalog = self.catalog.read()?;
        if title.is_some() || prompt.is_some() {
            catalog.update(id, title, prompt)?;
        }
        if let Some(enabled) = enabled {
            catalog.switch(id, enabled)?;
        }
        catalog.require(id)?;
        self.catalog.write(&catalog)?;
        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::InMemoryLensCatalog;

    #[test]
    fn the_words_and_the_switch_are_saved_in_one_write() {
        let update = UpdateLens::new(Arc::new(InMemoryLensCatalog::default()));
        let risk = LensId::parse("risk").unwrap();

        let catalog = update
            .execute(&risk, None, Some("Only the risk."), Some(false))
            .unwrap();

        let entry = catalog.require(&risk).unwrap();
        assert_eq!(entry.definition.prompt, "Only the risk.");
        assert!(
            !entry.enabled,
            "the panel sends both, and both land together"
        );
    }
}
