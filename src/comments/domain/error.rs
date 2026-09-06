//! What the comments refuse, in their own vocabulary.
//!
//! Separate from [`crate::error::Error`] for the same reason the map's is:
//! these are the things *this* layer decides, and the shared type should not
//! have to know what a line range or a pull request is.

#[derive(Debug, thiserror::Error)]
pub enum CommentError {
    #[error("a comment with no text says nothing")]
    Empty,

    #[error("no comment with id {id} — `farol comment list` shows them")]
    Unknown { id: String },

    #[error(
        "lines {from}-{to} of '{path}' are not in the diff\nA review comment can only sit where the diff reaches, so GitHub would refuse it."
    )]
    OutsideDiff { path: String, from: u32, to: u32 },

    // ---- publishing ------------------------------------------------------
    #[error(
        "these comments are no longer on lines the diff shows:\n{}\nThe branch moved under them. Close them, or write them again where the code went.",
        Listing(.comments)
    )]
    NoLongerInDiff { comments: Vec<String> },

    #[error("this repository has no remote, so there is no pull request to publish to")]
    NoRemote,

    #[error("farol has no token for GitHub yet")]
    NoToken,

    #[error(
        "the credential host changed\nRefresh the review and authorize the displayed host before saving a token."
    )]
    CredentialHostChanged,

    #[error("a review that asks for something has to say what — write the summary first")]
    NoSummary,

    #[error(
        "this review covers uncommitted work, and a pull request can only be reviewed at a commit\nCommit the change and derive the map again."
    )]
    Uncommitted,

    #[error(
        "GitHub would not answer with this token: it may have expired, or it may be missing one of the two permissions farol needs on this repository, Pull requests: Read and write and Contents: Read"
    )]
    TokenRefused,

    #[error("'{branch}' is not on GitHub yet — push it, then open a pull request for it")]
    BranchNotPushed { branch: String },

    #[error("'{branch}' has no pull request yet, and a review is posted onto one")]
    NoPullRequest { branch: String },

    #[error(
        "this pull request is yours, and GitHub only lets somebody else approve it or ask changes on it\nLeave a comment instead, or send the ticks on their own."
    )]
    OwnPullRequest,

    /// Why this is refused rather than sent: a comment anchors to a line
    /// number, and against another commit it lands on code nobody read. The
    /// message says none of that. Somebody reading it is standing in front of a
    /// review that will not go, and what they need is the one word that moves
    /// them on.
    #[error("{}: the pull request is at {theirs}, you are at {ours}", Fix(*.behind))]
    HeadMoved {
        theirs: String,
        ours: String,
        /// Whether the missing commits are ours to push or theirs to pull.
        behind: bool,
    },

    #[error("cannot reach {host}: {why}")]
    Unreachable { host: String, why: String },

    #[error("GitHub refused the review: {what}")]
    Refused { what: String },
}

/// One per line, indented, because the reviewer has to go and find each one.
struct Listing<'a>(&'a Vec<String>);

impl std::fmt::Display for Listing<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for one in self.0 {
            writeln!(f, "    {one}")?;
        }
        Ok(())
    }
}

/// Which way the branch has to move, since the reader cannot tell from two shas
/// which side is missing what.
struct Fix(bool);

impl std::fmt::Display for Fix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            true => write!(f, "Pull first"),
            false => write!(f, "Push first"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comment_off_the_diff_says_why_github_would_refuse_it() {
        let e = CommentError::OutsideDiff {
            path: "src/a.rs".into(),
            from: 82,
            to: 116,
        };

        let msg = e.to_string();
        assert!(msg.contains("82-116"), "{msg}");
        assert!(msg.contains("src/a.rs"), "{msg}");
        assert!(msg.contains("where the diff reaches"), "{msg}");
    }

    #[test]
    fn the_ones_that_drifted_are_listed_so_they_can_be_found() {
        let e = CommentError::NoLongerInDiff {
            comments: vec!["src/a.rs:82-116".into(), "src/b.rs:9".into()],
        };

        let msg = e.to_string();
        assert!(msg.contains("    src/a.rs:82-116"), "{msg}");
        assert!(msg.contains("    src/b.rs:9"), "{msg}");
    }

    #[test]
    fn a_moved_head_says_which_way_to_move() {
        // Two shas alone leave the reader guessing whose commits are missing.
        let ours = CommentError::HeadMoved {
            theirs: "abc1234".into(),
            ours: "def5678".into(),
            behind: false,
        };
        assert!(ours.to_string().starts_with("Push first"), "{ours}");

        let theirs = CommentError::HeadMoved {
            theirs: "abc1234".into(),
            ours: "def5678".into(),
            behind: true,
        };
        assert!(theirs.to_string().starts_with("Pull first"), "{theirs}");
        // Both shas stay: which one is which is the only thing the reader
        // cannot work out for themselves.
        assert!(theirs.to_string().contains("abc1234"), "{theirs}");
        assert!(theirs.to_string().contains("def5678"), "{theirs}");
    }
}
