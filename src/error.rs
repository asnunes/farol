//! What farol reports when it cannot do what was asked.
//!
//! The layers below own their refusals — [`MapError`] for the map, [`ScopeError`]
//! for the review window — because those are the things they decide. What is
//! left here belongs to no single layer: the repository not being in a state
//! farol can work with, and the world failing underneath.
//!
//! This type sits *above* the layers, not beneath them. It used to hold every
//! variant in the program, which made `shared` import `LineRange` from the map —
//! a dependency running the wrong way, in the module whose whole job is to be
//! depended on.

use crate::diff::domain::ScopeError;
use crate::map::domain::MapError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // ---- the repository is not in a workable state ----------------------
    #[error("HEAD is detached — check out a branch first")]
    DetachedHead,

    #[error("no base branch found — tried 'main' and 'master'")]
    NoBaseBranch,

    #[error(
        "--dirty only works on the branch you are standing on; head resolved to '{head}' but you are on '{current}'"
    )]
    DirtyOnOtherHead { head: String, current: String },

    // ---- refusals that belong to a layer --------------------------------
    #[error(transparent)]
    Map(#[from] MapError),

    #[error(transparent)]
    Scope(#[from] ScopeError),

    // ---- the world failing underneath -----------------------------------
    /// git failing, mostly. There is nothing farol can add to what the library
    /// said, so it says that and no more.
    #[error("{0}")]
    Message(String),

    /// A file farol was told to use by name. Worth its own variants because
    /// the operating system says "No such file or directory" and stops, while
    /// the one thing worth knowing — which file — is the part farol has.
    #[error("cannot read {path}: {source}")]
    CannotRead {
        path: String,
        source: std::io::Error,
    },

    #[error("cannot write {path}: {source}")]
    CannotWrite {
        path: String,
        source: std::io::Error,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn msg(m: impl Into<String>) -> Self {
        Error::Message(m.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layers_refusal_reads_as_itself_rather_than_as_a_wrapper() {
        // The caller sees the map's own message, not `Map(...)` around it.
        let e: Error = MapError::DuplicateBlock {
            slug: "core".into(),
        }
        .into();

        assert_eq!(
            e.to_string(),
            "block 'core' already exists — use `block update` to change it"
        );
    }

    #[test]
    fn a_rejected_path_reads_as_itself_too() {
        let e: Error = ScopeError::PathOutOfScope {
            path: "nowhere.rs".into(),
            similar: vec![],
        }
        .into();

        assert!(e.to_string().contains("nowhere.rs"), "{e}");
    }

    #[test]
    fn dirty_on_another_head_names_both_sides_of_the_contradiction() {
        let e = Error::DirtyOnOtherHead {
            head: "other".into(),
            current: "feature/x".into(),
        };

        let msg = e.to_string();
        assert!(msg.contains("other"), "{msg}");
        assert!(msg.contains("feature/x"), "{msg}");
    }

    #[test]
    fn every_variant_this_layer_owns_says_something() {
        for e in [
            Error::DetachedHead,
            Error::NoBaseBranch,
            Error::msg("cannot resolve 'nope'"),
            Error::CannotRead {
                path: "map-abc.farol.json".into(),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            },
            Error::CannotWrite {
                path: "map-abc.farol.json".into(),
                source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            },
        ] {
            assert!(!e.to_string().trim().is_empty(), "{e:?}");
        }
    }
}
