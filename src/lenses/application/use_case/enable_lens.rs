use std::sync::Arc;

use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensCatalogRepository, LensId};

#[derive(Clone)]
pub struct EnableLens {
    catalog: Arc<dyn LensCatalogRepository>,
}

impl EnableLens {
    pub fn new(catalog: Arc<dyn LensCatalogRepository>) -> Self {
        Self { catalog }
    }

    pub fn execute(&self, id: &LensId) -> Result<LensCatalog> {
        let mut catalog = self.catalog.read()?;
        catalog.switch(id, true)?;
        self.catalog.write(&catalog)?;
        Ok(catalog)
    }
}
