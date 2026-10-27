use std::sync::Arc;

use crate::error::Result;
use crate::lenses::domain::{LensCatalog, LensCatalogRepository};

#[derive(Clone)]
pub struct ListLenses {
    catalog: Arc<dyn LensCatalogRepository>,
}

impl ListLenses {
    pub fn new(catalog: Arc<dyn LensCatalogRepository>) -> Self {
        Self { catalog }
    }

    pub fn execute(&self) -> Result<LensCatalog> {
        self.catalog.read()
    }
}
