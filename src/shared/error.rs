use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "no map for branch {branch}\nRun the review-map skill in the session that implemented this change."
    )]
    NoMap { branch: String },

    #[error("unknown block '{slug}'\nExisting blocks: {}", Listing(.existing))]
    UnknownBlock { slug: String, existing: Vec<String> },

    #[error("block '{slug}' already exists — use `block update` to change it")]
    DuplicateBlock { slug: String },

    #[error("'{path}' is not part of this review{}", Suggestion(.similar))]
    PathOutOfScope { path: String, similar: Vec<String> },

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

    #[error("no line note at {range} on '{path}' in block '{slug}'")]
    NoSuchLineNote {
        slug: String,
        path: String,
        range: crate::map::domain::LineRange,
    },

    #[error(
        "no deactivated note at {range} on '{path}' in block '{slug}'\nRun `farol map derive` to see what is pending."
    )]
    NoSuchOrphan {
        slug: String,
        path: String,
        range: crate::map::domain::LineRange,
    },

    #[error("HEAD is detached — check out a branch first")]
    DetachedHead,

    #[error(
        "--dirty only works on the branch you are standing on; head resolved to '{head}' but you are on '{current}'"
    )]
    DirtyOnOtherHead { head: String, current: String },

    #[error("no base branch found — tried 'main' and 'master'")]
    NoBaseBranch,

    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn msg(m: impl Into<String>) -> Self {
        Error::Message(m.into())
    }
}

/// Renders a list inline, or says there are none.
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

/// Renders "did you mean" only when there is something to suggest.
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

    /// Every message is read by the session writing the map, so each one has to
    /// name what went wrong and leave a way forward.
    #[test]
    fn a_rejected_path_offers_the_near_misses() {
        let e = Error::PathOutOfScope {
            path: "src/stores/db.rs".into(),
            similar: vec!["src/store/db.rs".into()],
        };

        let msg = e.to_string();
        assert!(msg.contains("src/stores/db.rs"), "{msg}");
        assert!(msg.contains("Did you mean: src/store/db.rs"), "{msg}");
    }

    #[test]
    fn a_rejected_path_with_nothing_like_it_says_nothing_extra() {
        // Better silence than sending the author off to another wrong path.
        let e = Error::PathOutOfScope {
            path: "nowhere.py".into(),
            similar: vec![],
        };

        let msg = e.to_string();
        assert!(msg.contains("nowhere.py"), "{msg}");
        assert!(!msg.contains("Did you mean"), "{msg}");
    }

    #[test]
    fn an_unknown_block_lists_the_ones_that_exist() {
        let e = Error::UnknownBlock {
            slug: "nope".into(),
            existing: vec!["core".into(), "wiring".into()],
        };

        let msg = e.to_string();
        assert!(msg.contains("nope"), "{msg}");
        assert!(msg.contains("core, wiring"), "{msg}");
    }

    #[test]
    fn an_unknown_block_on_an_empty_map_says_there_are_none_yet() {
        // An empty list would read as a formatting bug rather than as an
        // answer.
        let e = Error::UnknownBlock {
            slug: "nope".into(),
            existing: vec![],
        };

        assert!(e.to_string().contains("(none yet)"), "{e}");
    }

    #[test]
    fn a_missing_map_names_the_branch_and_says_how_to_make_one() {
        let e = Error::NoMap {
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
        let e = Error::RangeOutOfFile {
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
    fn a_malformed_slug_shows_what_was_typed() {
        assert!(
            Error::BadSlug {
                raw: "Not A Slug".into()
            }
            .to_string()
            .contains("Not A Slug")
        );
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
    fn every_variant_says_something() {
        // A blank message would strand whoever hit it.
        for e in [
            Error::DetachedHead,
            Error::NoBaseBranch,
            Error::DuplicateBlock {
                slug: "core".into(),
            },
            Error::BadRange {
                raw: "10..20".into(),
            },
            Error::msg("something went wrong"),
        ] {
            assert!(!e.to_string().trim().is_empty(), "{e:?}");
        }
    }
}
