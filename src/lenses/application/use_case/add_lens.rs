use std::sync::Arc;

use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensCatalogRepository, LensGroup};

/// A lens the user made, at the end of its group.
#[derive(Clone)]
pub struct AddLens {
    catalog: Arc<dyn LensCatalogRepository>,
}

impl AddLens {
    pub fn new(catalog: Arc<dyn LensCatalogRepository>) -> Self {
        Self { catalog }
    }

    pub fn execute(&self, title: &str, group: LensGroup, prompt: &str) -> Result<LensCatalog> {
        let mut catalog = self.catalog.read()?;
        catalog.add(title, group, prompt)?;
        self.catalog.write(&catalog)?;
        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::InMemoryLensCatalog;

    #[test]
    fn an_added_lens_is_there_the_next_time_the_catalog_is_read() {
        let repo = Arc::new(InMemoryLensCatalog::default());

        AddLens::new(repo.clone())
            .execute("Migrations", LensGroup::File, "What each migration does.")
            .unwrap();

        let ids: Vec<_> = repo
            .read()
            .unwrap()
            .all()
            .into_iter()
            .map(|e| e.definition.id.to_string())
            .collect();
        assert!(ids.contains(&"migrations".to_string()), "{ids:?}");
    }
}
