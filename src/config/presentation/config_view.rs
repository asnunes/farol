use serde::Serialize;

use crate::config::domain::Config;

#[derive(Serialize)]
pub struct ConfigView {
    open_browser: bool,
    show_open_browser: bool,
}

impl From<Config> for ConfigView {
    fn from(config: Config) -> Self {
        Self {
            open_browser: config.open_browser,
            show_open_browser: cfg!(target_os = "macos"),
        }
    }
}
