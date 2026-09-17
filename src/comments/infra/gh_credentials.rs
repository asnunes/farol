use std::path::PathBuf;
use std::process::Command;

use crate::comments::domain::{CommentError, Credentials};
use crate::error::Result;

/// GitHub CLI owns authentication; Farol neither stores nor configures tokens.
pub struct GhCredentials {
    executable: PathBuf,
}

impl GhCredentials {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Credentials for GhCredentials {
    fn token(&self, host: &str) -> Result<Option<String>> {
        let output = Command::new(&self.executable)
            .args(["auth", "token", "--hostname", host])
            .output()
            .map_err(|_| CommentError::GitHubCliUnavailable)?;
        // gh may include account details in stderr. Keep diagnostics actionable
        // without reflecting authentication output back into a review session.
        if !output.status.success() {
            return Ok(None);
        }
        let token = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok((!token.is_empty()).then_some(token))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn the_requested_host_is_passed_to_gh_without_using_a_shell() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("gh");
        std::fs::write(&exe, "#!/bin/sh\n[ \"$1\" = auth ] && [ \"$2\" = token ] && [ \"$3\" = --hostname ] && [ \"$4\" = github.example.test ] || exit 1\nprintf 'fixture-token\\n'\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        let credentials = GhCredentials::new(exe);
        assert_eq!(
            credentials.token("github.example.test").unwrap().as_deref(),
            Some("fixture-token")
        );
        assert_eq!(credentials.token("different.example.test").unwrap(), None);
    }

    #[test]
    fn a_missing_cli_explains_what_to_install() {
        let dir = tempfile::tempdir().unwrap();
        let error = GhCredentials::new(dir.path().join("absent-gh"))
            .token("github.com")
            .unwrap_err();
        assert!(error.to_string().contains("Install GitHub CLI"));
    }

    #[test]
    fn a_failed_auth_command_does_not_expose_its_output() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("gh");
        std::fs::write(&exe, "#!/bin/sh\necho private-auth-detail >&2\nexit 1\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(GhCredentials::new(exe).token("github.com").unwrap(), None);
    }
}
