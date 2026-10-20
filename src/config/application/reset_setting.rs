use std::sync::Arc;

use crate::config::domain::{Config, ConfigRepository, SettingKey};
use crate::error::Result;

/// Put one setting back to its default, leaving the others as they were.
#[derive(Clone)]
pub struct ResetSetting {
    repo: Arc<dyn ConfigRepository>,
}

impl ResetSetting {
    pub fn new(repo: Arc<dyn ConfigRepository>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, key: SettingKey) -> Result<Config> {
        let mut config = self.repo.read()?;
        config.reset(key);
        self.repo.write(&config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::application::SetSetting;
    use crate::config::domain::SettingValue;
    use crate::testing::InMemoryConfigRepository;

    #[test]
    fn resetting_a_setting_brings_back_the_catalog_default() {
        let repo = Arc::new(InMemoryConfigRepository::default());
        let instruction = SettingKey::parse("send-instruction").unwrap();
        let browser = SettingKey::parse("open-browser").unwrap();
        SetSetting::new(repo.clone())
            .execute(instruction, SettingValue::Text("Only answer.".into()))
            .unwrap();
        SetSetting::new(repo.clone())
            .execute(browser, SettingValue::Boolean(false))
            .unwrap();

        let config = ResetSetting::new(repo).execute(instruction).unwrap();

        assert!(config.send_instruction().starts_with("Answer it"));
        assert!(
            !config.should_open(false, false),
            "resetting one setting must leave the others alone"
        );
    }
}
