use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub open_browser: bool,
}

impl Config {
    pub fn should_open(&self, open: bool, no_open: bool) -> bool {
        if no_open {
            return false;
        }
        open || self.open_browser
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { open_browser: true }
    }
}

pub trait ConfigRepository: Send + Sync {
    fn read(&self) -> Result<Config>;
    fn write(&self, config: &Config) -> Result<()>;
}
