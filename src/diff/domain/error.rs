//! What the review window refuses.
//!
//! One variant, because there is only one thing this layer decides on its own:
//! whether a path is part of the review. Everything else that can go wrong when
//! asking git is git failing, which is not a domain refusal.

use std::fmt;

/// What this layer's functions return.
pub type Result<T> = std::result::Result<T, ScopeError>;

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    /// The reader of this is the session writing the map, and a hallucinated
    /// path is the mistake it makes most — so the rejection carries the way out.
    #[error("'{path}' is not part of this review{}", Suggestion(.similar))]
    PathOutOfScope { path: String, similar: Vec<String> },
}

/// Renders "did you mean" only when there is something to suggest. Better
/// silence than sending the author off to another wrong path.
struct Suggestion<'a>(&'a Vec<String>);

impl fmt::Display for Suggestion<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            Ok(())
        } else {
            write!(f, "\nDid you mean: {}", self.0.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejected_path_offers_the_near_misses() {
        let e = ScopeError::PathOutOfScope {
            path: "src/stores/db.rs".into(),
            similar: vec!["src/store/db.rs".into()],
        };

        let msg = e.to_string();
        assert!(msg.contains("src/stores/db.rs"), "{msg}");
        assert!(msg.contains("Did you mean: src/store/db.rs"), "{msg}");
    }

    #[test]
    fn a_rejected_path_with_nothing_like_it_says_nothing_extra() {
        let e = ScopeError::PathOutOfScope {
            path: "nowhere.py".into(),
            similar: vec![],
        };

        let msg = e.to_string();
        assert!(msg.contains("nowhere.py"), "{msg}");
        assert!(!msg.contains("Did you mean"), "{msg}");
    }
}
