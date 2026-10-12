use std::sync::Arc;

use crate::config::domain::{Config, ConfigRepository};
use crate::error::Result;

#[derive(Clone)]
pub struct SetOpenBrowser {
    repo: Arc<dyn ConfigRepository>,
}

impl SetOpenBrowser {
    pub fn new(repo: Arc<dyn ConfigRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, open_browser: bool) -> Result<Config> {
        let mut config = self.repo.read()?;
        config.open_browser = open_browser;
        self.repo.write(&config)?;
        Ok(config)
    }
}
