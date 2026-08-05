//! The adapter: git, behind the three ports the rest of the program uses.
//!
//! It holds the window and answers questions about it. Talking to git is
//! [`Git`]'s job and building the window is [`Window`]'s; what is left here is
//! the shape of the ports.

use std::collections::{BTreeMap, BTreeSet};

use super::git::{Blob, Diffed, Git, Side};
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
