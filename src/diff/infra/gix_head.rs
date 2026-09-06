use super::git::Git;
use crate::diff::domain::{HeadSource, HeadState};
use crate::error::Result;

/// A fresh thread-local handle observes ref changes without building a diff.
pub struct GixHead {
    repo: gix::ThreadSafeRepository,
}

impl GixHead {
    pub fn new(repo: gix::Repository) -> Self {
        Self {
            repo: repo.into_sync(),
        }
    }
}

impl HeadSource for GixHead {
    fn read_head(&self) -> Result<HeadState> {
        Git::new(self.repo.to_thread_local()).head_state()
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::Fixture;
    use super::*;

    #[test]
    fn remote_updates_and_packing_refs_leave_the_current_head_unchanged() {
        let repo = Fixture::new();
        let source = GixHead::new(gix::open(repo.dir.path()).unwrap());
        let before = source.read_head().unwrap();
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo.git(&["tag", "v1"]);
        repo.git(&["branch", "other"]);
        repo.git(&["pack-refs", "--all", "--prune"]);
        assert_eq!(source.read_head().unwrap(), before);
        repo.git(&["commit", "--allow-empty", "-qm", "next"]);
        let current = source.read_head().unwrap();
        assert_ne!(current.commit, before.commit);
        repo.git(&["update-ref", "refs/heads/other", "HEAD"]);
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo.git(&["pack-refs", "--all", "--prune"]);
        assert_eq!(source.read_head().unwrap(), current);
        repo.git(&["checkout", "-q", "other"]);
        let switched = source.read_head().unwrap();
        assert_eq!(switched.commit, current.commit);
        assert_ne!(switched.reference, current.reference);
    }

    #[test]
    fn a_linked_worktree_reads_its_own_head_and_shared_branch_refs() {
        let repo = Fixture::new();
        let parent = tempfile::tempdir().unwrap();
        let worktree = parent.path().join("review");
        repo.git(&[
            "worktree",
            "add",
            "-qb",
            "review",
            worktree.to_str().unwrap(),
        ]);
        let source = GixHead::new(gix::open(&worktree).unwrap());
        let before = source.read_head().unwrap();
        assert_eq!(before.reference.as_deref(), Some("refs/heads/review"));
        repo.git(&["commit", "--allow-empty", "-qm", "main advances"]);
        assert_eq!(source.read_head().unwrap(), before);
        repo.git(&["update-ref", "refs/heads/review", "HEAD"]);
        assert_ne!(source.read_head().unwrap().commit, before.commit);
        let updated = source.read_head().unwrap();
        repo.git(&["pack-refs", "--all", "--prune"]);
        assert_eq!(source.read_head().unwrap(), updated);
    }
}
