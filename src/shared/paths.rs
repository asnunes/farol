use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::shared::error::{Error, Result};

/// The repository farol was invoked in, resolved once.
///
/// Owning discovery, the branch name and the git dir together keeps callers
/// from re-deriving any of them — and from assembling the git dir by hand,
/// which is the mistake that breaks inside a worktree.
pub struct Workspace {
    repo: gix::Repository,
    branch: String,
    git_dir: PathBuf,
}

impl Workspace {
    /// Open the repository containing `start`.
    pub fn discover(start: &Path) -> Result<Self> {
        let repo = gix::discover(start)
            .map_err(|e| Error::msg(format!("not inside a git repository: {e}")))?;
        let branch = Self::branch_of(&repo)?;
        let git_dir = repo.git_dir().to_path_buf();
        Ok(Self {
            repo,
            branch,
            git_dir,
        })
    }

    pub fn here() -> Result<Self> {
        Self::discover(&std::env::current_dir()?)
    }

    pub fn repo(&self) -> &gix::Repository {
        &self.repo
    }

    pub fn into_repo(self) -> gix::Repository {
        self.repo
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// Where farol keeps everything for this branch.
    pub fn store(&self) -> Store {
        Store::new(&self.git_dir, &self.branch)
    }

    /// Branch the worktree is on. Detached HEAD has no name to key state on, so
    /// it is refused rather than silently keyed by sha.
    fn branch_of(repo: &gix::Repository) -> Result<String> {
        let head = repo
            .head()
            .map_err(|e| Error::msg(format!("cannot read HEAD: {e}")))?;
        match head.referent_name() {
            Some(name) => Ok(name.shorten().to_string()),
            None => Err(Error::DetachedHead),
        }
    }
}

/// Everything farol stores for one branch.
///
/// Deliberately under the *worktree's own* git dir rather than the shared one:
/// removing a worktree means the feature is done, and taking the review state
/// with it is free cleanup instead of a graveyard of dead branches.
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(git_dir: &Path, branch: &str) -> Self {
        Self {
            root: git_dir.join("farol").join(Self::sanitize(branch)),
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

    /// Serialise and write so that a crash never leaves half a file behind.
    ///
    /// The write goes to a sibling temp file, is flushed to disk, then renamed
    /// over the target. Rename within a filesystem is atomic, so the old copy
    /// survives intact until the new one is complete. Review state accumulates
    /// over days; losing it to a mistimed Ctrl-C is not acceptable.
    pub fn write_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        use std::io::Write;

        self.ensure()?;
        let body = serde_json::to_vec_pretty(value)?;
        let tmp = path.with_extension("tmp");
        {
            let mut file = std::fs::File::create(&tmp)?;
            file.write_all(&body)?;
            file.sync_all()?;
        }
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Read and parse, treating anything unreadable as absent. `on_stale` is
    /// called with the reason so the caller can say so out loud — silence would
    /// look like the review had simply never been mapped.
    pub fn read_json<T: DeserializeOwned>(
        &self,
        path: &Path,
        on_stale: impl FnOnce(String),
    ) -> Option<T> {
        let raw = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str::<T>(&raw) {
            Ok(value) => Some(value),
            Err(e) => {
                on_stale(e.to_string());
                None
            }
        }
    }

    /// Branch names carry slashes; file names should not.
    fn sanitize(branch: &str) -> String {
        branch
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                other => other,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slashes_in_branch_names_become_dashes() {
        assert_eq!(
            Store::sanitize("fix/retry-on-timeout"),
            "fix-retry-on-timeout"
        );
        assert_eq!(Store::sanitize("feat/a/b"), "feat-a-b");
        assert_eq!(Store::sanitize("main"), "main");
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
    fn writing_replaces_content_and_leaves_no_temp_behind() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path(), "b");
        let target = store.map_file("abc");

        store.write_json(&target, &vec![1, 2, 3]).unwrap();
        let back: Vec<i32> = store.read_json(&target, |_| ()).unwrap();
        assert_eq!(back, vec![1, 2, 3]);

        store.write_json(&target, &vec![9]).unwrap();
        let back: Vec<i32> = store.read_json(&target, |_| ()).unwrap();
        assert_eq!(back, vec![9]);
        assert!(!target.with_extension("tmp").exists());
    }

    #[test]
    fn unreadable_json_reports_why_and_reads_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path(), "b");
        let target = store.map_file("abc");
        store.ensure().unwrap();
        std::fs::write(&target, "{ truncated").unwrap();

        let mut reason = None;
        let back: Option<Vec<i32>> = store.read_json(&target, |e| reason = Some(e));
        assert!(back.is_none());
        assert!(
            reason.is_some(),
            "the caller must be told, not left guessing"
        );
    }
}
