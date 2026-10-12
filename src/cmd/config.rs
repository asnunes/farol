use clap::Subcommand;

use crate::config::domain::SettingKey;
use crate::config::presentation::ConfigView;
use crate::error::Result;

#[derive(Subcommand)]
pub(super) enum ConfigAction {
    /// Show the values saved for this user on this machine.
    Show,
    /// List available settings with their labels, defaults and current values.
    List,
    /// Save a global preference. Use config list to discover available keys.
    Set {
        key: String,
        #[arg(action = clap::ArgAction::Set)]
        value: bool,
    },
}

impl ConfigAction {
    pub(super) fn run(self) -> Result<()> {
        let cases = super::wiring::config_cases()?;
        let output = match self {
            Self::List => serde_json::to_string_pretty(&ConfigView::from(cases.get.execute()?))?,
            Self::Show => serde_json::to_string_pretty(&cases.get.execute()?)?,
            Self::Set { key, value } => {
                let config = cases.set_setting.execute(SettingKey::parse(&key)?, value)?;
                serde_json::to_string_pretty(&config)?
            }
        };
        println!("{output}");
        Ok(())
    }
}
