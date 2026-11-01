//! Reaching the host as the reviewer.

use crate::error::Result;

/// Credentials are read only for the selected host; authentication is managed externally.
///
/// A port for one reason: publishing is tested against a fake, and a fake that
/// consulted the real login would read the person's actual token.
pub trait Credentials: Send + Sync {
    fn token(&self, host: &str) -> Result<Option<String>>;
}
