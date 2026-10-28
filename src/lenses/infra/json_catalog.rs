use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::lenses::domain::{LensCatalog, LensCatalogRepository};
use crate::shared::paths::write_json;

/// The user's lenses, in a file of their own beside the settings: a list of
/// definitions is not a setting, and the settings file refuses keys it does
/// not know.
pub struct JsonLensCatalog {
    directory: PathBuf,
}

impl JsonLensCatalog {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
}

impl LensCatalogRepository for JsonLensCatalog {
    fn read(&self) -> Result<LensCatalog> {
        let path = self.directory.join("lenses.json");
        let body = match std::fs::read(&path) {
            Ok(body) => body,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LensCatalog::default());
            }
            Err(error) => {
                return Err(Error::msg(format!(
                    "cannot read {}: {error}. Check the file permissions.",
                    path.display()
                )));
            }
        };
        serde_json::from_slice(&body).map_err(|error| {
            Error::msg(format!(
                "invalid lenses at {}: {error}. Fix it in Settings, or delete the file to go back to the lenses farol ships.",
                path.display()
            ))
        })
    }

    fn write(&self, catalog: &LensCatalog) -> Result<()> {
        std::fs::create_dir_all(&self.directory)?;
        write_json(&self.directory.join("lenses.json"), catalog)
    }
}
