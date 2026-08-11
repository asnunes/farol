//! What the map refuses, in the map's own vocabulary.
//!
//! Separate from [`crate::error::Error`] because these are the things
//! *this* layer knows how to be wrong about. Keeping them in the shared type
//! made the shared layer import `LineRange` from here, which is the dependency
//! running the wrong way.

use std::fmt;

use super::range::LineRange;

/// Every message is read by the session writing the map, so each one names what
/// went wrong and leaves a way forward.
/// What this layer's functions return.
pub type Result<T> = std::result::Result<T, MapError>;

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    #[error(
        "no map for branch {branch}\nRun the farol skill in the session that implemented this change."
    )]
    NoMap { branch: String },

    #[error("unknown block '{slug}'\nExisting blocks: {}", Listing(.existing))]
    UnknownBlock { slug: String, existing: Vec<String> },

    #[error("block '{slug}' already exists — use `block update` to change it")]
    DuplicateBlock { slug: String },

    #[error("'{path}' is not in block '{slug}'\nFiles in that block: {}", Listing(.existing))]
    PathNotInBlock {
        slug: String,
        path: String,
        existing: Vec<String>,
    },

    #[error("'{path}' is already in block '{slug}'")]
    DuplicatePath { slug: String, path: String },

    #[error("lines {from}-{to} are outside '{path}', which has {total} lines")]
    RangeOutOfFile {
        path: String,
        from: u32,
        to: u32,
        total: u32,
    },

    #[error(
        "invalid block name '{raw}' — expected lowercase words joined by dashes, for example recover-link"
    )]
    BadSlug { raw: String },

    #[error("invalid range '{raw}' — expected <from>-<to>, for example 82-116")]
    BadRange { raw: String },

    #[error("'{path}' is not marked as skim")]
    NotSkimmed { path: String },

    #[error("no line note at {range} on '{path}' in block '{slug}'")]
    NoSuchLineNote {
        slug: String,
        path: String,
        range: LineRange,
    },

    #[error(
        "no deactivated note at {range} on '{path}' in block '{slug}'\nRun `farol map derive` to see what is pending."
    )]
    NoSuchOrphan {
        slug: String,
        path: String,
        range: LineRange,
    },
}

/// Renders a list inline, or says there are none. An empty list would read as a
/// formatting bug rather than as an answer.
struct Listing<'a>(&'a Vec<String>);

impl fmt::Display for Listing<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "(none yet)")
        } else {
            write!(f, "{}", self.0.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_block_lists_the_ones_that_exist() {
        let e = MapError::UnknownBlock {
            slug: "nope".into(),
            existing: vec!["core".into(), "wiring".into()],
        };

        let msg = e.to_string();
        assert!(msg.contains("nope"), "{msg}");
        assert!(msg.contains("core, wiring"), "{msg}");
    }

    #[test]
    fn an_unknown_block_on_an_empty_map_says_there_are_none_yet() {
        let e = MapError::UnknownBlock {
            slug: "nope".into(),
            existing: vec![],
        };

        assert!(e.to_string().contains("(none yet)"), "{e}");
    }

    #[test]
    fn a_missing_map_names_the_branch_and_says_how_to_make_one() {
        let e = MapError::NoMap {
            branch: "feature/x".into(),
        };

        let msg = e.to_string();
        assert!(msg.contains("feature/x"), "{msg}");
        assert!(
            msg.contains("skill"),
            "the way out has to be in the message: {msg}"
        );
    }

    #[test]
    fn a_range_past_the_end_says_how_long_the_file_actually_is() {
        let e = MapError::RangeOutOfFile {
            path: "a.rs".into(),
            from: 200,
            to: 210,
            total: 100,
        };

        let msg = e.to_string();
        assert!(msg.contains("200-210"), "{msg}");
        assert!(msg.contains("100 lines"), "{msg}");
    }

    #[test]
    fn a_deactivated_note_that_is_not_there_says_where_to_look() {
        let e = MapError::NoSuchOrphan {
            slug: "core".into(),
            path: "a.rs".into(),
            range: LineRange::new(10, 12).unwrap(),
        };

        assert!(e.to_string().contains("farol map derive"), "{e}");
    }

    #[test]
    fn every_variant_says_something() {
        for e in [
            MapError::DuplicateBlock {
                slug: "core".into(),
            },
            MapError::DuplicatePath {
                slug: "core".into(),
                path: "a.rs".into(),
            },
            MapError::BadSlug {
                raw: "Not A Slug".into(),
            },
            MapError::BadRange {
                raw: "10..20".into(),
            },
            MapError::PathNotInBlock {
                slug: "core".into(),
                path: "a.rs".into(),
                existing: vec![],
            },
            MapError::NoSuchLineNote {
                slug: "core".into(),
                path: "a.rs".into(),
                range: LineRange::new(1, 2).unwrap(),
            },
        ] {
            assert!(!e.to_string().trim().is_empty(), "{e:?}");
        }
    }
}
