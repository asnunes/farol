use std::path::PathBuf;
use std::process::Command;

use crate::comments::domain::{BranchPusher, CommentError};
use crate::error::Result;

/// `git push --set-upstream` from the working tree, with whatever credentials
/// the reviewer's git already uses: farol never sees them.
pub struct GitPush {
    repo: PathBuf,
}

impl GitPush {
    pub fn new(repo: PathBuf) -> Self {
        Self { repo }
    }
}

impl BranchPusher for GitPush {
    fn push(&self, branch: &str) -> Result<()> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args(["push", "--set-upstream", "origin", branch])
            .output()?;
        match out.status.success() {
            true => Ok(()),
            false => Err(CommentError::PushRefused {
                branch: branch.to_string(),
                why: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            }
            .into()),
        }
    }
}
