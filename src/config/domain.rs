use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// What the user set on purpose, and nothing else.
///
/// Defaults are not written down. A file that held every default would freeze
/// them the first time any one setting was saved, and a better default shipped
/// later would never reach whoever had once switched the browser off.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(try_from = "BTreeMap<String, SettingValue>")]
pub struct Config(BTreeMap<String, SettingValue>);

impl Config {
    pub fn should_open(&self, open: bool, no_open: bool) -> bool {
        if no_open {
            return false;
        }
        open || self.flag(&OPEN_BROWSER)
    }

    /// What follows the comment when it is sent to the agent.
    pub fn send_instruction(&self) -> &str {
        self.text(&SEND_INSTRUCTION)
    }

    /// What the agent follows when it writes a pending comment.
    pub fn pending_instruction(&self) -> &str {
        self.text(&PENDING_INSTRUCTION)
    }

    pub fn value(&self, definition: &SettingDefinition) -> SettingValue {
        self.0
            .get(definition.key)
            .cloned()
            .unwrap_or_else(|| definition.kind.default())
    }

    /// Every setting as it is in effect, defaults included — what a person
    /// asking "what is configured" wants, which is not what the file holds.
    pub fn effective(&self) -> BTreeMap<&'static str, SettingValue> {
        SettingDefinition::all()
            .iter()
            .map(|definition| (definition.key, self.value(definition)))
            .collect()
    }

    pub fn set(
        &mut self,
        key: SettingKey,
        value: SettingValue,
    ) -> std::result::Result<(), ConfigError> {
        key.0.kind.accept(key.0.key, &value)?;
        self.0.insert(key.0.key.to_string(), value);
        Ok(())
    }

    /// Back to the default, by forgetting the choice rather than writing the
    /// default down in its place.
    pub fn reset(&mut self, key: SettingKey) {
        self.0.remove(key.0.key);
    }

    fn flag(&self, definition: &SettingDefinition) -> bool {
        match self.0.get(definition.key) {
            Some(SettingValue::Boolean(value)) => *value,
            _ => matches!(definition.kind, SettingKind::Boolean { default: true }),
        }
    }

    fn text(&self, definition: &SettingDefinition) -> &str {
        match (self.0.get(definition.key), &definition.kind) {
            (Some(SettingValue::Text(value)), _) => value,
            (_, SettingKind::Text { default }) => default,
            _ => "",
        }
    }
}

impl TryFrom<BTreeMap<String, SettingValue>> for Config {
    type Error = ConfigError;

    fn try_from(values: BTreeMap<String, SettingValue>) -> std::result::Result<Self, ConfigError> {
        let mut config = Self::default();
        for (key, value) in values {
            config.set(SettingKey::parse(&key)?, value)?;
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

    /// A value typed on the command line, read the way this setting reads.
    ///
    /// Text is taken as it came, `false` included: the shell cannot say which
    /// was meant, and the setting can.
    pub fn parse_value(&self, raw: &str) -> std::result::Result<SettingValue, ConfigError> {
        match self.0.kind {
            SettingKind::Boolean { .. } => match raw {
                "true" => Ok(SettingValue::Boolean(true)),
                "false" => Ok(SettingValue::Boolean(false)),
                _ => Err(ConfigError::WrongKind {
                    key: self.0.key,
                    expected: "true or false",
                }),
            },
            SettingKind::Text { .. } => Ok(SettingValue::Text(raw.to_string())),
        }
    }
}

/// A setting's value as it is stored and sent: a JSON boolean or a JSON string.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SettingValue {
    Boolean(bool),
    Text(String),
}

#[derive(Debug)]
pub struct SettingDefinition {
    pub key: &'static str,
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub kind: SettingKind,
    pub visible: bool,
}

impl SettingDefinition {
    pub fn all() -> &'static [Self] {
        &[OPEN_BROWSER, SEND_INSTRUCTION, PENDING_INSTRUCTION]
    }
}

/// What a setting holds, with its default — one declaration, so the type and
/// the default cannot disagree.
#[derive(Debug)]
pub enum SettingKind {
    Boolean { default: bool },
    Text { default: &'static str },
}

impl SettingKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Boolean { .. } => "boolean",
            Self::Text { .. } => "text",
        }
    }

    pub fn default(&self) -> SettingValue {
        match self {
            Self::Boolean { default } => SettingValue::Boolean(*default),
            Self::Text { default } => SettingValue::Text(default.to_string()),
        }
    }

    fn accept(
        &self,
        key: &'static str,
        value: &SettingValue,
    ) -> std::result::Result<(), ConfigError> {
        match (self, value) {
            (Self::Boolean { .. }, SettingValue::Boolean(_))
            | (Self::Text { .. }, SettingValue::Text(_)) => Ok(()),
            (Self::Boolean { .. }, _) => Err(ConfigError::WrongKind {
                key,
                expected: "true or false",
            }),
            (Self::Text { .. }, _) => Err(ConfigError::WrongKind {
                key,
                expected: "text",
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unknown setting '{0}'. Run farol config list for available keys.")]
    UnknownSetting(String),

    #[error("setting '{key}' takes {expected}. Run farol config list to see each setting's type.")]
    WrongKind {
        key: &'static str,
        expected: &'static str,
    },
}

pub trait ConfigRepository: Send + Sync {
    fn read(&self) -> Result<Config>;
    fn write(&self, config: &Config) -> Result<()>;
}

const OPEN_BROWSER: SettingDefinition = SettingDefinition {
    key: "open_browser",
    title: "Open browser automatically",
    description: None,
    kind: SettingKind::Boolean { default: true },
    visible: cfg!(target_os = "macos"),
};

const SEND_INSTRUCTION: SettingDefinition = SettingDefinition {
    key: "send_instruction",
    title: "Instruction sent with each comment",
    description: Some(
        "Added after the comment when you press Send or Ask. Farol always includes the comment's location, text and id. Leave it empty to send only the comment.",
    ),
    kind: SettingKind::Text {
        default: "Answer it, or change the code and say what you changed.",
    },
    visible: true,
};

const PENDING_INSTRUCTION: SettingDefinition = SettingDefinition {
    key: "pending_instruction",
    title: "Instruction for generated pending comments",
    description: Some(
        "What the agent follows when you press Generate pending comment. Farol always includes the thread and how to write the comment.",
    ),
    kind: SettingKind::Text {
        default: "Write it as a review comment for the author of the pull request: say what to change and why, kindly and briefly, without repeating the conversation.",
    },
    visible: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_definition_provides_a_default_and_accepts_cli_and_json_keys() {
        for definition in SettingDefinition::all() {
            let config = Config::default();
            assert_eq!(config.value(definition), definition.kind.default());
            let key = SettingKey::parse(&definition.key.replace('_', "-")).unwrap();
            let mut changed = Config::default();
            let other = match definition.kind {
                SettingKind::Boolean { default } => SettingValue::Boolean(!default),
                SettingKind::Text { .. } => SettingValue::Text("something else".into()),
            };
            changed.set(key, other.clone()).unwrap();
            assert_eq!(changed.value(definition), other);
            assert!(SettingKey::parse(definition.key).is_ok());
        }
    }

    #[test]
    fn a_file_written_before_text_settings_still_reads() {
        let config: Config = serde_json::from_str(r#"{"open_browser":false}"#).unwrap();
        assert!(!config.should_open(false, false));
        assert_eq!(
            config.send_instruction(),
            "Answer it, or change the code and say what you changed."
        );
    }

    #[test]
    fn a_value_of_the_wrong_kind_is_refused_naming_the_key() {
        for body in [
            r#"{"open_browser":"false"}"#,
            r#"{"send_instruction":true}"#,
        ] {
            let error = serde_json::from_str::<Config>(body)
                .unwrap_err()
                .to_string();
            assert!(error.contains("takes"), "{body}: {error}");
        }
        assert!(serde_json::from_str::<Config>(r#"{"unknown":false}"#).is_err());
    }

    #[test]
    fn saving_one_setting_does_not_freeze_the_defaults_of_the_others() {
        let mut config = Config::default();
        config
            .set(
                SettingKey::parse("open-browser").unwrap(),
                SettingValue::Boolean(false),
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(&config).unwrap(),
            serde_json::json!({"open_browser": false}),
            "a default shipped later has to reach whoever never chose the instruction"
        );
    }

    #[test]
    fn an_empty_instruction_is_kept_apart_from_an_unset_one() {
        let key = SettingKey::parse("send-instruction").unwrap();
        let mut config = Config::default();
        config.set(key, SettingValue::Text(String::new())).unwrap();
        assert_eq!(config.send_instruction(), "");

        config.reset(key);
        assert_eq!(
            config.send_instruction(),
            SEND_INSTRUCTION.kind.default_text()
        );
        assert_eq!(
            serde_json::to_value(&config).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn text_typed_on_the_command_line_is_taken_as_it_came() {
        let text = SettingKey::parse("send-instruction").unwrap();
        assert_eq!(
            text.parse_value("false").unwrap(),
            SettingValue::Text("false".into())
        );
        let flag = SettingKey::parse("open-browser").unwrap();
        assert!(flag.parse_value("maybe").is_err());
    }

    impl SettingKind {
        fn default_text(&self) -> &'static str {
            match self {
                Self::Text { default } => default,
                Self::Boolean { .. } => "",
            }
        }
    }
}
