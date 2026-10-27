use std::sync::Arc;

use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensCatalogRepository, LensId};

/// Change a lens's title or prompt.
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
    ) -> Result<LensCatalog> {
        let mut catalog = self.catalog.read()?;
        catalog.update(id, title, prompt)?;
        self.catalog.write(&catalog)?;
        Ok(catalog)
    }
}
