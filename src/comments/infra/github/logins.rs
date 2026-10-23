//! Who a token belongs to, remembered for the life of the process.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;

use crate::error::Result;

/// The login behind each token, asked of the host once.
///
/// Every look at the pull request needs it, to tell whether the pull request
/// is the reader's own, and asking `/user` each time cost a third of a second
/// on every reload. The answer only changes when the token does, so it is
/// keyed by the token — by a hash of it, so the token itself is kept nowhere
/// but where `gh` keeps it.
///
/// One per server rather than one per request: the adapter is rebuilt with
/// every request, and a cache inside it would forget between them.
#[derive(Default)]
pub struct Logins(Mutex<HashMap<u64, String>>);

impl Logins {
    pub fn get_or(&self, token: &str, ask: impl FnOnce() -> Result<String>) -> Result<String> {
        let key = hashed(token);
        if let Some(known) = self.0.lock().unwrap().get(&key) {
            return Ok(known.clone());
        }
        let login = ask()?;
        // An empty answer is not remembered: it is the host not saying, and
        // the next look should ask again.
        if !login.is_empty() {
            self.0.lock().unwrap().insert(key, login.clone());
        }
        Ok(login)
    }
}

fn hashed(token: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_is_asked_once_per_token() {
        let logins = Logins::default();
        let mut asked = 0;

        for _ in 0..3 {
            let login = logins
                .get_or("token-a", || {
                    asked += 1;
                    Ok("alice".into())
                })
                .unwrap();
            assert_eq!(login, "alice");
        }
        assert_eq!(asked, 1);

        let other = logins.get_or("token-b", || Ok("bob".into())).unwrap();
        assert_eq!(other, "bob", "a login with another account is asked again");
    }
}
