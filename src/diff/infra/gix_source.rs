//! The adapter: git, behind the three ports the rest of the program uses.
//!
//! It holds the window and answers questions about it. Talking to git is
//! [`Git`]'s job and building the window is [`Window`]'s; what is left here is
//! the shape of the ports.

use std::collections::{BTreeMap, BTreeSet};

use super::blob::{Blob, Diffed, Side};
use super::git::Git;
use super::window::Window;
use crate::diff::domain::{
    CommitHistorySource, FileDiff, FileDiffSource, FileStatus, ReviewScopeSource, Scope,
};
use crate::shared::error::{Error, Result};

/// What the caller asked for on the command line, before resolution.
#[derive(Debug, Clone, Default)]
pub struct ScopeRequest {
    pub base: Option<String>,
    pub head: Option<String>,
    pub direct: bool,
    pub dirty: bool,
}

pub struct GixSource {
    /// Stored in the thread-safe form: a plain `Repository` carries `RefCell`
    /// caches and cannot cross threads, but axum handlers need `Sync`.
    repo: gix::ThreadSafeRepository,
    scope: Scope,
    /// path -> blob on each side, for the review window only.
    base_blobs: BTreeMap<String, Blob>,
    head_blobs: BTreeMap<String, Blob>,
    /// Paths whose new side is uncommitted work: there is no object to hand
    /// git, so the diff has to point it at the working tree instead.
    from_worktree: BTreeSet<String>,
}

impl GixSource {
    /// The branch comes from the workspace rather than being rediscovered here:
    /// one place decides what branch we are on, and it already refused a
    /// detached HEAD.
    pub fn open(repo: gix::Repository, current: &str, req: &ScopeRequest) -> Result<Self> {
        let git = Git::new(repo);
        let current = current.to_string();

        let base_ref = match &req.base {
            Some(b) => b.clone(),
            None => git.default_base()?,
        };
        let head_ref = req.head.clone().unwrap_or_else(|| current.clone());

        // Uncommitted work belongs to the tree you are standing in; asking for
        // it while pointing head somewhere else is a contradiction, not a
        // detail to paper over.
        if req.dirty && head_ref != current {
            return Err(Error::DirtyOnOtherHead {
                head: head_ref,
                current,
            });
        }

        let head_id = git.resolve(&head_ref)?;
        let base_tip = git.resolve(&base_ref)?;
        let base_id = if req.direct {
            base_tip
        } else {
            git.merge_base(base_tip, head_id)?
        };

        let window = Window::of(&git, base_id, head_id, req.dirty)?;

        let scope = Scope {
            branch: current,
            base_ref,
            head_ref,
            base_sha: base_id.to_hex().to_string(),
            head_sha: head_id.to_hex().to_string(),
            merge_base: !req.direct,
            dirty: req.dirty,
            files: window.files,
        };

        Ok(Self {
            repo: git.into_sync(),
            scope,
            base_blobs: window.base_blobs,
            head_blobs: window.head_blobs,
            from_worktree: window.from_worktree,
        })
    }

    /// A handle for this thread. Cheap — it shares the object database and only
    /// rebuilds the local caches.
    fn git(&self) -> Git {
        Git::new(self.repo.to_thread_local())
    }
}

impl ReviewScopeSource for GixSource {
    fn scope(&self) -> Result<&Scope> {
        Ok(&self.scope)
    }

    fn file_line_count(&self, path: &str) -> Result<u32> {
        let blob = self
            .head_blobs
            .get(path)
            .ok_or_else(|| self.scope.reject(path))?;
        Ok(String::from_utf8_lossy(&blob.data).lines().count() as u32)
    }
}

impl FileDiffSource for GixSource {
    fn file_diff(&self, path: &str) -> Result<FileDiff> {
        let change = self
            .scope
            .files
            .iter()
            .find(|f| f.path == path)
            .ok_or_else(|| self.scope.reject(path))?;

        let old_key = change.old_path.clone().unwrap_or_else(|| path.to_string());
        let old = self.base_blobs.get(&old_key);
        let new = self.head_blobs.get(path);

        let diffed = self.git().diff(
            path,
            old.map(Side::Object).unwrap_or(Side::Absent),
            self.new_side(path, new),
        )?;

        Ok(Self::assemble(
            path,
            change.old_path.clone(),
            change.status,
            new.map(Blob::hash).unwrap_or_default(),
            diffed,
        ))
    }

    fn content_hash(&self, path: &str) -> Result<String> {
        self.head_blobs
            .get(path)
            .map(Blob::hash)
            .ok_or_else(|| self.scope.reject(path))
    }

    fn file_diff_between(&self, from: &str, to: &str, path: &str) -> Result<Option<FileDiff>> {
        let git = self.git();
        let from_id = git.resolve(from)?;
        let old = git.blob_at(from_id, path)?;

        let new = if to == crate::shared::WORKING {
            git.worktree_blob(path)?
        } else {
            let to_id = git.resolve(to)?;
            git.blob_at(to_id, path)?
        };

        // Identical content is the same blob, so git's id settles it without
        // comparing the bytes.
        if old.as_ref().map(|b| b.id) == new.as_ref().map(|b| b.id) {
            return Ok(None);
        }

        let diffed = git.diff(
            path,
            old.as_ref().map(Side::Object).unwrap_or(Side::Absent),
            if to == crate::shared::WORKING {
                Side::Worktree
            } else {
                new.as_ref().map(Side::Object).unwrap_or(Side::Absent)
            },
        )?;

        Ok(Some(Self::assemble(
            path,
            None,
            FileStatus::Modified,
            new.as_ref().map(Blob::hash).unwrap_or_default(),
            diffed,
        )))
    }
}

impl GixSource {
    fn new_side<'a>(&self, path: &str, blob: Option<&'a Blob>) -> Side<'a> {
        match blob {
            None => Side::Absent,
            Some(_) if self.from_worktree.contains(path) => Side::Worktree,
            Some(blob) => Side::Object(blob),
        }
    }

    fn assemble(
        path: &str,
        old_path: Option<String>,
        status: FileStatus,
        new_content_hash: String,
        diffed: Diffed,
    ) -> FileDiff {
        let (binary, hunks, additions, deletions) = match diffed {
            Diffed::Untouchable => (true, Vec::new(), 0, 0),
            Diffed::Text(h) => (false, h.hunks, h.additions, h.deletions),
        };
        FileDiff {
            path: path.to_string(),
            old_path,
            status,
            hunks,
            binary,
            additions,
            deletions,
            new_content_hash,
        }
    }
}

impl CommitHistorySource for GixSource {
    fn head_sha(&self) -> Result<String> {
        Ok(self.scope.head_sha.clone())
    }

    fn commits_ahead_of(&self, sha: &str) -> Result<u32> {
        if sha == crate::shared::WORKING {
            return Ok(0);
        }
        let git = self.git();
        let target = git.resolve(sha)?;
        let head = git.resolve(&self.scope.head_sha)?;
        git.commits_between(target, head)
    }

    fn is_ancestor(&self, sha: &str) -> Result<bool> {
        if sha == crate::shared::WORKING {
            return Ok(false);
        }
        let git = self.git();
        let Ok(target) = git.resolve(sha) else {
            return Ok(false);
        };
        let head = git.resolve(&self.scope.head_sha)?;
        Ok(git.is_ancestor(target, head))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::domain::LineKind;
    use crate::diff::infra::fixture::Fixture;

    fn numbered(lines: usize) -> String {
        (1..=lines).map(|i| format!("line {i}\n")).collect()
    }

    /// A branch that edits one line of a forty-line file.
    fn edited() -> Fixture {
        let f = Fixture::new();
        f.write("src/a.rs", &numbered(40));
        f.commit("add a");
        f.on_branch("feature/x");
        f.write("src/a.rs", &numbered(40).replace("line 20\n", "CHANGED\n"));
        f.commit("edit a");
        f
    }

    #[test]
    fn the_diff_a_reviewer_reads_carries_the_line_numbers_the_notes_anchor_to() {
        let f = edited();
        let diff = f
            .source("feature/x", ScopeRequest::default())
            .file_diff("src/a.rs")
            .unwrap();

        assert!(!diff.binary);
        assert_eq!((diff.additions, diff.deletions), (1, 1));
        let added = diff.hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == LineKind::Added)
            .expect("the edited line should be there");
        assert_eq!(added.content, "CHANGED");
        assert_eq!(added.new_number, Some(20));
    }

    #[test]
    fn a_renamed_file_is_diffed_against_the_path_it_came_from() {
        // The old side lives under the old name. Reading it under the new one
        // would show the whole file as added, which is the reading the rename
        // detection exists to prevent.
        let f = Fixture::new();
        f.write("old/a.rs", &numbered(40));
        f.commit("add a");
        f.on_branch("feature/x");
        std::fs::create_dir_all(f.dir.path().join("new")).unwrap();
        f.git(&["mv", "old/a.rs", "new/a.rs"]);
        f.write("new/a.rs", &numbered(40).replace("line 20\n", "CHANGED\n"));
        f.commit("move and edit");

        let diff = f
            .source("feature/x", ScopeRequest::default())
            .file_diff("new/a.rs")
            .unwrap();

        assert_eq!(diff.old_path.as_deref(), Some("old/a.rs"));
        assert_eq!(
            (diff.additions, diff.deletions),
            (1, 1),
            "one line changed, not forty: {:?}",
            diff.hunks.len()
        );
    }

    #[test]
    fn a_file_outside_the_window_is_refused_by_both_ways_in() {
        let f = edited();
        let source = f.source("feature/x", ScopeRequest::default());

        assert!(source.file_diff("README.md").is_err());
        assert!(source.content_hash("README.md").is_err());
    }

    #[test]
    fn the_content_hash_is_the_blob_id_git_gives_the_new_side() {
        // Viewed state is keyed on this. If it drifted from git, every file
        // would reopen on a rebase that changed nothing.
        let f = edited();
        let expected = f.git(&["rev-parse", "HEAD:src/a.rs"]);

        assert_eq!(
            f.source("feature/x", ScopeRequest::default())
                .content_hash("src/a.rs")
                .unwrap(),
            expected
        );
    }

    #[test]
    fn under_dirty_the_new_side_comes_off_disk() {
        let f = edited();
        f.write(
            "src/a.rs",
            &numbered(40).replace("line 20\n", "UNCOMMITTED\n"),
        );

        let diff = f
            .source(
                "feature/x",
                ScopeRequest {
                    dirty: true,
                    ..Default::default()
                },
            )
            .file_diff("src/a.rs")
            .unwrap();

        assert!(
            diff.hunks[0]
                .lines
                .iter()
                .any(|l| l.kind == LineKind::Added && l.content == "UNCOMMITTED"),
            "the object database has no such blob yet"
        );
    }

    #[test]
    fn a_binary_file_arrives_with_no_hunks_and_says_why() {
        let f = Fixture::new();
        f.on_branch("feature/x");
        std::fs::write(
            f.dir.path().join("logo.png"),
            (0u8..=255).cycle().take(4000).collect::<Vec<u8>>(),
        )
        .unwrap();
        f.commit("add a binary");

        let diff = f
            .source("feature/x", ScopeRequest::default())
            .file_diff("logo.png")
            .unwrap();

        assert!(diff.binary);
        assert!(diff.hunks.is_empty());
        assert_eq!((diff.additions, diff.deletions), (0, 0));
    }

    #[test]
    fn a_deleted_file_is_all_deletions() {
        let f = Fixture::new();
        f.write("gone.rs", &numbered(5));
        f.commit("add it");
        f.on_branch("feature/x");
        f.git(&["rm", "-q", "gone.rs"]);
        f.commit("remove it");

        let diff = f
            .source("feature/x", ScopeRequest::default())
            .file_diff("gone.rs")
            .unwrap();

        assert_eq!((diff.additions, diff.deletions), (0, 5));
        assert!(
            diff.hunks[0]
                .lines
                .iter()
                .all(|l| l.kind == LineKind::Removed)
        );
    }

    // ---- what deriving asks of it ---------------------------------------

    #[test]
    fn the_head_it_resolved_is_the_one_it_reports() {
        let f = edited();
        let expected = f.git(&["rev-parse", "HEAD"]);

        let source = f.source("feature/x", ScopeRequest::default());

        assert_eq!(source.head_sha().unwrap(), expected);
    }

    #[test]
    fn a_file_identical_between_two_commits_has_no_diff_to_show() {
        // Deriving asks this of every file with a note on it; answering with
        // an empty diff would make every note look like it needed moving.
        let f = edited();
        let head = f.git(&["rev-parse", "HEAD"]);

        let out = f
            .source("feature/x", ScopeRequest::default())
            .file_diff_between(&head, &head, "src/a.rs")
            .unwrap();

        assert!(out.is_none());
    }

    #[test]
    fn a_file_that_changed_between_two_commits_comes_back_with_the_hunks() {
        let f = edited();
        let head = f.git(&["rev-parse", "HEAD"]);
        let base = f.git(&["rev-parse", "HEAD~1"]);

        let diff = f
            .source("feature/x", ScopeRequest::default())
            .file_diff_between(&base, &head, "src/a.rs")
            .unwrap()
            .expect("the file changed between them");

        assert_eq!((diff.additions, diff.deletions), (1, 1));
    }

    #[test]
    fn the_working_tree_can_be_the_far_side_of_a_comparison() {
        // How a map written against uncommitted work is brought forward.
        let f = edited();
        let head = f.git(&["rev-parse", "HEAD"]);
        f.write(
            "src/a.rs",
            &numbered(40).replace("line 20\n", "UNCOMMITTED\n"),
        );

        let diff = f
            .source(
                "feature/x",
                ScopeRequest {
                    dirty: true,
                    ..Default::default()
                },
            )
            .file_diff_between(&head, crate::shared::WORKING, "src/a.rs")
            .unwrap()
            .expect("the working tree differs from the commit");

        assert!(
            diff.hunks[0]
                .lines
                .iter()
                .any(|l| l.content == "UNCOMMITTED")
        );
    }

    #[test]
    fn uncommitted_work_is_never_behind_and_never_an_ancestor() {
        // It is not a commit, so the questions history answers do not apply.
        let f = edited();
        let source = f.source("feature/x", ScopeRequest::default());

        assert_eq!(source.commits_ahead_of(crate::shared::WORKING).unwrap(), 0);
        assert!(!source.is_ancestor(crate::shared::WORKING).unwrap());
    }

    #[test]
    fn a_commit_that_cannot_be_resolved_is_not_an_ancestor_rather_than_an_error() {
        // A map left behind by a branch that was rebased away names a commit
        // this repository no longer has.
        let f = edited();

        assert!(
            !f.source("feature/x", ScopeRequest::default())
                .is_ancestor("0000000000000000000000000000000000000000")
                .unwrap()
        );
    }
}
