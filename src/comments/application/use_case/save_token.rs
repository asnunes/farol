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
}

impl SaveToken {
    pub fn new(credentials: Arc<dyn Credentials>) -> Self {
        Self { credentials }
    }

    /// Nothing is echoed back and nothing is logged: the token goes in and the
    /// answer is that it went in.
    pub fn execute(&self, token: &str) -> Result<()> {
        match token.trim().is_empty() {
            true => Err(CommentError::NoToken.into()),
            false => self.credentials.set(token),
        }
    }
}
