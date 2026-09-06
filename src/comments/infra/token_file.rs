//! The GitHub token, in a file only its owner can read.

use std::path::{Path, PathBuf};

use crate::comments::domain::Credentials;
use crate::error::{Error, Result};
use crate::shared::paths::config_dir;

/// A token on disk under farol's configuration directory.
///
/// Read on every use rather than held: a token revoked in the browser should
/// stop working here without restarting the server, and a token pasted into a
/// farol that has been running all afternoon should start working at once.
pub struct TokenFile {
    path: PathBuf,
}

impl TokenFile {
    /// Under `$XDG_CONFIG_HOME/farol`, where a person can find and delete it.
    pub fn here() -> Result<Self> {
        Ok(Self::at(config_dir()?.join(NAME)))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Credentials for TokenFile {
    fn token(&self, host: &str) -> Result<Option<String>> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(Error::CannotRead {
                    path: self.path.display().to_string(),
                    source,
                });
            }
        };
        if raw.trim().is_empty() {
            return Ok(None);
        }
        let saved: SavedToken = serde_json::from_str(&raw).map_err(|_| {
            Error::msg("cannot read the saved credential\nSave a token for this host again in the review page.")
        })?;
        if saved.host != host || saved.token.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(saved.token.trim().to_string()))
    }

    fn set(&self, host: &str, token: &str) -> Result<()> {
        let body = serde_json::to_vec(&SavedToken {
            host: host.to_string(),
            token: token.trim().to_string(),
        })?;
        let write = || -> std::io::Result<()> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Created empty with the mode already narrowed, then written: a
            // file opened world-readable and chmodded afterwards is readable
            // for as long as that takes.
            let mut file = restricted(&self.path)?;
            std::io::Write::write_all(&mut file, &body)?;
            file.sync_all()
        };
        write().map_err(|source| Error::CannotWrite {
            path: self.path.display().to_string(),
            source,
        })
    }
}

// Unbound tokens cannot authorize a host; this file records the explicit choice.
const NAME: &str = "github-credential.json";

#[cfg(unix)]
fn restricted(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn restricted(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::File::create(path)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedToken {
    host: String,
    token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_token_is_available_only_to_its_authorized_host() {
        let dir = tempfile::tempdir().unwrap();
        let file = TokenFile::at(dir.path().join(NAME));
        file.set("github.com", "test-secret").unwrap();
        let reopened = TokenFile::at(dir.path().join(NAME));
        assert_eq!(
            reopened.token("github.com").unwrap().as_deref(),
            Some("test-secret")
        );
        for host in [
            "enterprise.example",
            "github.com.attacker.example",
            "attacker.example",
        ] {
            assert_eq!(
                reopened.token(host).unwrap(),
                None,
                "a saved credential must not authorize {host}"
            );
        }
        reopened
            .set("enterprise.example", "enterprise-secret")
            .unwrap();
        assert_eq!(file.token("github.com").unwrap(), None);
        assert_eq!(
            file.token("enterprise.example").unwrap().as_deref(),
            Some("enterprise-secret")
        );
    }

    #[test]
    fn an_unbound_file_does_not_authorize_any_host() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("github-token"), "unbound-secret").unwrap();
        let file = TokenFile::at(dir.path().join(NAME));
        assert_eq!(file.token("github.com").unwrap(), None);
        assert_eq!(file.token("enterprise.example").unwrap(), None);
    }

    #[test]
    fn unreadable_credentials_do_not_echo_secret_material() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(NAME);
        std::fs::write(&path, r#"{"host": "github.com", "token": ["test-secret"]}"#).unwrap();
        let error = TokenFile::at(path)
            .token("github.com")
            .unwrap_err()
            .to_string();
        assert!(!error.contains("test-secret"));
        assert!(error.contains("Save a token"), "{error}");
    }

    #[test]
    fn a_token_that_was_never_written_reads_as_absent_rather_than_as_an_error() {
        // The whole flow hangs off this: no token is the first thing the screen
        // explains, not something that fails a request.
        let dir = tempfile::tempdir().unwrap();
        let file = TokenFile::at(dir.path().join("github-token"));

        assert_eq!(file.token("github.com").unwrap(), None);
    }

    #[test]
    fn a_token_comes_back_without_the_newline_it_was_pasted_with() {
        let dir = tempfile::tempdir().unwrap();
        let file = TokenFile::at(dir.path().join("nested/github-token"));

        file.set("github.com", "ghp_abc123\n").unwrap();

        assert_eq!(
            file.token("github.com").unwrap().as_deref(),
            Some("ghp_abc123")
        );
    }

    #[test]
    fn a_file_holding_only_whitespace_is_no_token_at_all() {
        // Emptying the file is how a person revokes it locally, and it should
        // read the same as never having written one.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github-token");
        std::fs::write(&path, "  \n").unwrap();

        assert_eq!(TokenFile::at(&path).token("github.com").unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn nobody_else_on_the_machine_can_read_it() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github-token");
        TokenFile::at(&path)
            .set("github.com", "ghp_abc123")
            .unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode is {mode:o}");
    }

    #[test]
    fn writing_a_second_token_leaves_none_of_the_first_behind() {
        // The longer token was written first, so a truncation that did not
        // happen would leave its tail on the end of the shorter one.
        let dir = tempfile::tempdir().unwrap();
        let file = TokenFile::at(dir.path().join("github-token"));
        file.set("github.com", "ghp_a_very_long_token_indeed")
            .unwrap();

        file.set("github.com", "ghp_short").unwrap();

        assert_eq!(
            file.token("github.com").unwrap().as_deref(),
            Some("ghp_short")
        );
    }
}
