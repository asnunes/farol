use serde::Serialize;

use crate::config::domain::{Config, SettingDefinition};

#[derive(Serialize)]
pub struct ConfigView {
    settings: Vec<SettingView>,
}

impl From<Config> for ConfigView {
    fn from(config: Config) -> Self {
        Self {
            settings: SettingDefinition::all()
                .iter()
                .filter(|definition| definition.visible)
                .map(|definition| SettingView {
                    key: definition.key,
                    title: definition.title,
                    description: definition.description,
                    kind: "boolean",
                    default: definition.default,
                    value: config.value(definition),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct SettingView {
    key: &'static str,
    title: &'static str,
    description: Option<&'static str>,
    #[serde(rename = "type")]
    kind: &'static str,
    default: bool,
    value: bool,
}
