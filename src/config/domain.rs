use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "BTreeMap<String, bool>")]
pub struct Config(BTreeMap<String, bool>);

impl Config {
    pub fn should_open(&self, open: bool, no_open: bool) -> bool {
        if no_open {
            return false;
        }
        open || self.value(&OPEN_BROWSER)
    }

    pub fn value(&self, definition: &SettingDefinition) -> bool {
        self.0
            .get(definition.key)
            .copied()
            .unwrap_or(definition.default)
    }

    pub fn set(&mut self, key: SettingKey, value: bool) {
        self.0.insert(key.0.key.to_string(), value);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self(
            SettingDefinition::all()
                .iter()
                .map(|setting| (setting.key.to_string(), setting.default))
                .collect(),
        )
    }
}

impl TryFrom<BTreeMap<String, bool>> for Config {
    type Error = ConfigError;

    fn try_from(values: BTreeMap<String, bool>) -> std::result::Result<Self, ConfigError> {
        let mut config = Self::default();
        for (key, value) in values {
            let definition = SettingDefinition::all()
                .iter()
                .find(|setting| setting.key == key)
                .ok_or_else(|| ConfigError::UnknownSetting(key.clone()))?;
            config.set(SettingKey(definition), value);
        }
        Ok(config)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SettingKey(&'static SettingDefinition);

impl SettingKey {
    pub fn parse(raw: &str) -> std::result::Result<Self, ConfigError> {
        let key = raw.replace('-', "_");
        SettingDefinition::all()
            .iter()
            .find(|setting| setting.key == key)
            .map(Self)
            .ok_or_else(|| ConfigError::UnknownSetting(raw.to_string()))
    }
}

#[derive(Debug)]
pub struct SettingDefinition {
    pub key: &'static str,
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub default: bool,
    pub visible: bool,
}

impl SettingDefinition {
    pub fn all() -> &'static [Self] {
        &[OPEN_BROWSER]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unknown setting '{0}'. Run farol config list for available keys.")]
    UnknownSetting(String),
}

pub trait ConfigRepository: Send + Sync {
    fn read(&self) -> Result<Config>;
    fn write(&self, config: &Config) -> Result<()>;
}

const OPEN_BROWSER: SettingDefinition = SettingDefinition {
    key: "open_browser",
    title: "Open browser automatically",
    description: None,
    default: true,
    visible: cfg!(target_os = "macos"),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_definition_provides_a_default_and_accepts_cli_and_json_keys() {
        for definition in SettingDefinition::all() {
            let mut config = Config::default();
            assert_eq!(config.value(definition), definition.default);
            let key = SettingKey::parse(&definition.key.replace('_', "-")).unwrap();
            config.set(key, !definition.default);
            assert_eq!(config.value(definition), !definition.default);
            assert!(SettingKey::parse(definition.key).is_ok());
        }
    }

    #[test]
    fn stored_values_are_plain_json_and_missing_values_use_the_catalog_defaults() {
        let config: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            serde_json::json!({"open_browser": true})
        );
        assert!(serde_json::from_str::<Config>(r#"{"unknown":false}"#).is_err());
        assert!(serde_json::from_str::<Config>(r#"{"open_browser":"false"}"#).is_err());
    }
}
