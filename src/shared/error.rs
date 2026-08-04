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
