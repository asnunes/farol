use std::path::PathBuf;

use crate::config::domain::{Config, ConfigRepository};
use crate::error::{Error, Result};
use crate::shared::paths::write_json;

pub struct JsonConfigRepository {
    directory: PathBuf,
}

impl JsonConfigRepository {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
}

impl ConfigRepository for JsonConfigRepository {
    fn read(&self) -> Result<Config> {
        let path = self.directory.join("config.json");
        let body = match std::fs::read(&path) {
            Ok(body) => body,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Config::default());
            }
            Err(error) => {
                return Err(Error::msg(format!(
                    "cannot read {}: {error}. Check the file permissions.",
                    path.display()
                )));
            }
        };
        serde_json::from_slice(&body).map_err(|error| Error::msg(format!(
            "invalid configuration at {}: {error}. Set open_browser to true or false in this JSON file.", path.display()
        )))
    }

    fn write(&self, config: &Config) -> Result<()> {
        std::fs::create_dir_all(&self.directory)?;
        write_json(&self.directory.join("config.json"), config)
    }
}
