use std::sync::Arc;

use crate::config::domain::{Config, ConfigRepository};
use crate::error::Result;

#[derive(Clone)]
pub struct GetConfig {
    repo: Arc<dyn ConfigRepository>,
}

impl GetConfig {
    pub fn new(repo: Arc<dyn ConfigRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(&self) -> Result<Config> {
        self.repo.read()
    }
}
