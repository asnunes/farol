//! What the comments refuse, in their own vocabulary.
//!
//! Separate from [`crate::error::Error`] for the same reason the map's is:
//! these are the things *this* layer decides, and the shared type should not
//! have to know what a line range or a pull request is.

use crate::diff::domain::Side;

#[derive(Debug, thiserror::Error)]
pub enum CommentError {
    #[error("a comment with no text says nothing")]
    Empty,

    #[error("no thread with id {id} — `farol thread list` shows them")]
    Unknown { id: String },

    #[error(
        "lines {from}-{to} on the {side} side of '{path}' are not in the diff\nA review comment can only sit where the diff reaches, so GitHub would refuse it."
    )]
    OutsideDiff {
        path: String,
        /// Named, because the same numbers are in the diff on one side and not
        /// on the other, and without it the refusal reads as simply wrong.
        side: Side,
        from: u32,
        to: u32,
    },

    // ---- publishing ------------------------------------------------------
    #[error("this repository has no remote, so there is no pull request to publish to")]
    NoRemote,

    #[error(
        "no token available from GitHub CLI for this host\nRun `gh auth login --hostname HOST` with the repository host, then try again."
    )]
    NoToken,

    #[error(
        "cannot run gh\nInstall GitHub CLI and authenticate with `gh auth login --hostname HOST` for the repository host."
    )]
    GitHubCliUnavailable,

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

    #[error(
        "more than one open pull request shows this commit: {}\nfarol will not pick one. Close the ones that are not this review, or check out the branch of the one that is.",
        Numbers(.numbers)
    )]
    AmbiguousPullRequest { numbers: Vec<u32> },

    #[error(
        "thread {id} is on lines the diff no longer shows, so it cannot have a comment for the pull request\nGitHub only takes a review comment on a line of the diff. Start a thread where the code went."
    )]
    OffTheDiff { id: String },

    #[error(
        "thread {id} is about {about}, not lines of code, so it cannot have a comment for the pull request\nGitHub only takes a review comment on a line of the diff; answer it in farol instead."
    )]
    NotOnCode { id: String, about: String },

    #[error("could not push {branch} to origin: {why}\nPush it from a terminal, then try again.")]
    PushRefused { branch: String, why: String },

    #[error("a pull request is already open for {branch}: #{number}")]
    AlreadyOpen { branch: String, number: u32 },

    #[error("a pull request needs a title.")]
    NoTitle,

    #[error(
        "{id} is not a pending comment on the pull request\nRun `farol github status` for the ids of the pending ones."
    )]
    NoPending { id: String },

    #[error(
        "the pending comment of {id} was already submitted with a review, so it can no longer change"
    )]
    AlreadyPublished { id: String },
}

/// Pull request numbers as a person reads them: `#4, #7`.
struct Numbers<'a>(&'a Vec<u32>);

impl std::fmt::Display for Numbers<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let all: Vec<String> = self.0.iter().map(|n| format!("#{n}")).collect();
        write!(f, "{}", all.join(", "))
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
            side: Side::Old,
            from: 82,
            to: 116,
        };

        let msg = e.to_string();
        assert!(msg.contains("82-116"), "{msg}");
        assert!(msg.contains("src/a.rs"), "{msg}");
        assert!(msg.contains("where the diff reaches"), "{msg}");
        // Which side, because those very numbers may well be in the diff on
        // the other one — and then a refusal that did not say would read as
        // farol being wrong.
        assert!(msg.contains("old side"), "{msg}");
    }

    #[test]
    fn an_ambiguous_commit_names_every_pull_request_it_could_be() {
        let e = CommentError::AmbiguousPullRequest {
            numbers: vec![4, 7],
        };
        assert!(e.to_string().contains("#4, #7"), "{e}");
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
