use std::path::{Path, PathBuf};

use crate::shared::error::{Error, Result};

/// Everything farol stores for one branch.
///
/// Deliberately under the *worktree's own* git dir rather than the shared one:
/// removing a worktree means the feature is done, and taking the review state
/// with it is free cleanup instead of a graveyard of dead branches.
///
/// The path is asked of git, never built by hand — inside a worktree `.git` is
/// a file pointing elsewhere, and joining onto it would try to create a
/// directory inside a regular file.
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(git_dir: &Path, branch: &str) -> Self {
        Self {
            root: git_dir.join("farol").join(sanitize_branch(branch)),
        }
    }

    pub fn maps_dir(&self) -> PathBuf {
        self.root.join("maps")
    }

    pub fn map_file(&self, sha: &str) -> PathBuf {
        self.maps_dir().join(format!("{sha}.json"))
    }

    pub fn state_file(&self) -> PathBuf {
        self.root.join("state.json")
    }

    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(self.maps_dir())?;
        Ok(())
    }
}

/// Branch names carry slashes; file names should not.
pub fn sanitize_branch(branch: &str) -> String {
    branch
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other => other,
        })
        .collect()
}

/// Write a file so that a crash never leaves a half-written one behind: write
/// a sibling temp file, flush it to disk, then rename over the target. Rename
/// within a filesystem is atomic, so the old copy survives intact until the new
/// one is complete. Review state accumulates over days — losing it to a
/// mistimed Ctrl-C is not acceptable.
pub fn write_atomic(target: &Path, contents: &[u8]) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("tmp");
    {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, target)?;
    Ok(())
}

/// Open the repository containing `start`.
pub fn discover(start: &Path) -> Result<gix::Repository> {
    gix::discover(start).map_err(|e| Error::msg(format!("not inside a git repository: {e}")))
}

/// Branch the worktree is on. Detached HEAD has no name to key state on, so it
/// is refused rather than silently keyed by sha.
pub fn current_branch(repo: &gix::Repository) -> Result<String> {
    let head = repo
        .head()
        .map_err(|e| Error::msg(format!("cannot read HEAD: {e}")))?;
    match head.referent_name() {
        Some(name) => Ok(name.shorten().to_string()),
        None => Err(Error::DetachedHead),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slashes_in_branch_names_become_dashes() {
        assert_eq!(sanitize_branch("fix/bull-signing"), "fix-bull-signing");
        assert_eq!(sanitize_branch("feat/a/b"), "feat-a-b");
        assert_eq!(sanitize_branch("main"), "main");
    }

    #[test]
    fn store_paths_hang_off_the_given_git_dir() {
        let store = Store::new(Path::new("/repo/.git"), "feat/x");
        assert_eq!(store.maps_dir(), Path::new("/repo/.git/farol/feat-x/maps"));
        assert_eq!(
            store.map_file("abc123"),
            Path::new("/repo/.git/farol/feat-x/maps/abc123.json")
        );
        assert_eq!(
            store.state_file(),
            Path::new("/repo/.git/farol/feat-x/state.json")
        );
    }

    #[test]
    fn atomic_write_replaces_content_and_leaves_no_temp_behind() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested").join("f.json");

        write_atomic(&target, b"first").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "first");

        write_atomic(&target, b"second").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "second");
        assert!(!target.with_extension("tmp").exists());
    }
}
