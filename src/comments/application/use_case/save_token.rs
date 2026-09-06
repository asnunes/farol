use std::sync::Arc;

use crate::comments::domain::{CommentError, Credentials};
use crate::error::Result;

/// Keep the token farol will speak to the host with.
///
/// The one step of the whole flow that is not delegated to a session: a
/// credential is the person's, and handing an agent the job of putting it
/// somewhere is handing it the credential.
pub struct SaveToken {
    credentials: Arc<dyn Credentials>,
    host: Option<String>,
}

impl SaveToken {
    pub fn new(credentials: Arc<dyn Credentials>, host: Option<String>) -> Self {
        Self { credentials, host }
    }

    /// Nothing is echoed back and nothing is logged: the token goes in and the
    /// answer is that it went in.
    pub fn execute(&self, host: &str, token: &str) -> Result<()> {
        let expected = self.host.as_deref().ok_or(CommentError::NoRemote)?;
        if host != expected {
            return Err(CommentError::CredentialHostChanged.into());
        }
        match token.trim().is_empty() {
            true => Err(CommentError::NoToken.into()),
            false => self.credentials.set(host, token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeCredentials;

    #[test]
    fn saving_authorizes_only_the_host_displayed_to_the_user() {
        let credentials = Arc::new(FakeCredentials::default());
        let save = SaveToken::new(credentials.clone(), Some("github.com".into()));
        save.execute("github.com", "test-secret").unwrap();
        assert_eq!(
            credentials.token("github.com").unwrap().as_deref(),
            Some("test-secret")
        );
        assert_eq!(credentials.token("another.example").unwrap(), None);
    }

    #[test]
    fn a_changed_remote_cannot_reassign_the_submitted_token() {
        let credentials = Arc::new(FakeCredentials::default());
        credentials.set("github.com", "existing-secret").unwrap();
        let save = SaveToken::new(credentials.clone(), Some("another.example".into()));
        let error = save.execute("github.com", "new-secret").unwrap_err();
        assert!(error.to_string().contains("host changed"), "{error}");
        assert_eq!(credentials.token("another.example").unwrap(), None);
        assert_eq!(
            credentials.token("github.com").unwrap().as_deref(),
            Some("existing-secret")
        );
    }

    #[test]
    fn a_missing_remote_or_empty_token_cannot_authorize_a_host() {
        let credentials = Arc::new(FakeCredentials::default());
        let unhosted = SaveToken::new(credentials.clone(), None);
        assert!(unhosted.execute("github.com", "test-secret").is_err());
        let save = SaveToken::new(credentials.clone(), Some("github.com".into()));
        assert!(save.execute("github.com", "  ").is_err());
        assert_eq!(credentials.token("github.com").unwrap(), None);
    }
}
