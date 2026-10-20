use clap::Subcommand;

use crate::config::domain::SettingKey;
use crate::config::presentation::ConfigView;
use crate::error::Result;

#[derive(Subcommand)]
pub(super) enum ConfigAction {
    /// Show the values in effect for this user on this machine.
    Show,
    /// List available settings with their labels, types, defaults and current values.
    List,
    /// Save a global preference. Use config list to discover available keys.
    Set {
        key: String,
        /// `true` or `false` for a switch; the text as written for a text setting.
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    /// Put a setting back to its default.
    Reset { key: String },
}

impl ConfigAction {
    pub(super) fn run(self) -> Result<()> {
        let cases = super::wiring::config_cases()?;
        let output = match self {
            Self::List => serde_json::to_string_pretty(&ConfigView::from(cases.get.execute()?))?,
            Self::Show => serde_json::to_string_pretty(&cases.get.execute()?.effective())?,
            Self::Set { key, value } => {
                let key = SettingKey::parse(&key)?;
                let config = cases.set_setting.execute(key, key.parse_value(&value)?)?;
                serde_json::to_string_pretty(&config.effective())?
            }
            Self::Reset { key } => {
                let config = cases.reset_setting.execute(SettingKey::parse(&key)?)?;
                serde_json::to_string_pretty(&config.effective())?
            }
        };
        println!("{output}");
        Ok(())
    }
}
