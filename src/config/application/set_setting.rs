use std::sync::Arc;

use crate::config::domain::{Config, ConfigRepository, SettingKey, SettingValue};
use crate::error::Result;

#[derive(Clone)]
pub struct SetSetting {
    repo: Arc<dyn ConfigRepository>,
}

impl SetSetting {
    pub fn new(repo: Arc<dyn ConfigRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, key: SettingKey, value: SettingValue) -> Result<Config> {
        let mut config = self.repo.read()?;
        config.set(key, value)?;
        self.repo.write(&config)?;
        Ok(config)
    }
}
