use std::sync::Arc;

use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensCatalogRepository, LensId};

/// Remove a lens for good, one farol ships included.
#[derive(Clone)]
pub struct DeleteLens {
    catalog: Arc<dyn LensCatalogRepository>,
}

impl DeleteLens {
    pub fn new(catalog: Arc<dyn LensCatalogRepository>) -> Self {
        Self { catalog }
    }

    pub fn execute(&self, id: &LensId) -> Result<LensCatalog> {
        let mut catalog = self.catalog.read()?;
        catalog.delete(id)?;
        self.catalog.write(&catalog)?;
        Ok(catalog)
    }
}
