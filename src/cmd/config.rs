use clap::{Subcommand, ValueEnum};

use crate::error::Result;

#[derive(Subcommand)]
pub(super) enum ConfigAction {
    /// Show global preferences for this user on this machine.
    Show,
    /// Save a global preference.
    Set {
        key: ConfigKey,
        #[arg(action = clap::ArgAction::Set)]
        value: bool,
    },
}

impl ConfigAction {
    pub(super) fn run(self) -> Result<()> {
        let cases = super::wiring::config_cases()?;
        let config = match self {
            Self::Show => cases.get.execute()?,
            Self::Set {
                key: ConfigKey::OpenBrowser,
                value,
            } => cases.set_open_browser.execute(value)?,
        };
        println!("{}", serde_json::to_string_pretty(&config)?);
        Ok(())
    }
}

#[derive(Clone, ValueEnum)]
pub(super) enum ConfigKey {
    OpenBrowser,
}
