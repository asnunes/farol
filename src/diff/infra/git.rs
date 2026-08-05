//! The repository, and the questions farol asks it.
//!
//! Everything that speaks `gix` lives here, so the rest of the layer works in
//! object ids and blobs and never learns which library answers. The repository
//! is held, not passed: a caller gets a `Git` and asks it things, rather than
//! threading a `&Repository` through every function it calls.

use std::path::Path;

use crate::shared::error::{Error, Result};

/// A file's bytes together with git's own name for them.
///
/// The id is the blob's object id — read straight off the tree entry, not
/// computed. It is what viewed-state keys on, and what tells two versions of a
/// file apart without comparing them byte by byte.
pub(super) struct Blob {
    pub data: Vec<u8>,
    pub id: gix::ObjectId,
}

impl Blob {
    pub fn hash(&self) -> String {
        self.id.to_hex().to_string()
    }
}

/// Where one side of a diff comes from.
pub(super) enum Side<'a> {
    /// The file does not exist on this side — an addition or a deletion.
    Absent,
    Object(&'a Blob),
    /// Uncommitted work, which has no object to point at yet.
    Worktree,
}

impl Side<'_> {
    /// A null id means "no object": with a worktree root set for this side git
    /// reads the file from disk, and without one the side simply is not there.
    /// The empty blob would be wrong — it is an object, and a fresh repository
    /// has never stored it.
    fn id(&self, hash: gix::hash::Kind) -> gix::ObjectId {
        match self {
            Side::Absent | Side::Worktree => gix::ObjectId::null(hash),
            Side::Object(blob) => blob.id,
        }
    }
}

/// What came back from asking git to diff a file.
pub(super) enum Diffed {
    /// git will not diff it: binary content, or `-diff` in `.gitattributes`.
    Untouchable,
    Text(super::text_diff::Hunks),
}

pub(super) struct Git {
    repo: gix::Repository,
}

impl Git {
    pub fn new(repo: gix::Repository) -> Self {
        Self { repo }
    }

    pub fn into_sync(self) -> gix::ThreadSafeRepository {
        self.repo.into_sync()
    }

    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    // ---- revisions ------------------------------------------------------

    /// `main`, then `master`. Repos that predate the rename are still common
    /// enough that failing on the first try would be a daily annoyance.
    pub fn default_base(&self) -> Result<String> {
        for candidate in ["main", "master"] {
            if self.resolve(candidate).is_ok() {
                return Ok(candidate.to_string());
            }
        }
        Err(Error::NoBaseBranch)
    }

    pub fn resolve(&self, rev: &str) -> Result<gix::ObjectId> {
        self.repo
            .rev_parse_single(rev)
            .map(|id| id.detach())
            .map_err(|e| Error::msg(format!("cannot resolve '{rev}': {e}")))
    }

    pub fn merge_base(&self, a: gix::ObjectId, b: gix::ObjectId) -> Result<gix::ObjectId> {
        self.repo
            .merge_base(a, b)
            .map(|id| id.detach())
            .map_err(|e| Error::msg(format!("cannot find merge base: {e}")))
    }

    /// How many commits `head` is ahead of `target` — `target..head`, which is
    /// what `git rev-list --count` counts. Hiding the target stops the walk at
    /// it instead of reading all of history and needing an arbitrary cap to
    /// protect against never finding it.
    pub fn commits_between(&self, target: gix::ObjectId, head: gix::ObjectId) -> Result<u32> {
        let walk = self
            .repo
            .rev_walk([head])
            .with_hidden([target])
            .all()
            .map_err(|e| Error::msg(format!("cannot walk history: {e}")))?;

        let mut n = 0u32;
        for info in walk {
            info.map_err(|e| Error::msg(format!("cannot walk history: {e}")))?;
            n += 1;
        }
        Ok(n)
    }

    /// `git merge-base --is-ancestor`: the merge base of an ancestor with its
    /// descendant is the ancestor itself. Walking to look for it read the whole
    /// history to answer "no"; this stops at the base. Unrelated histories have
    /// no base at all, which is also a no.
    pub fn is_ancestor(&self, target: gix::ObjectId, head: gix::ObjectId) -> bool {
        self.merge_base(target, head)
            .map(|base| base == target)
            .unwrap_or(false)
    }

    // ---- objects --------------------------------------------------------

    pub fn tree(&self, id: gix::ObjectId) -> Result<gix::Tree<'_>> {
        self.repo
            .find_object(id)
            .map_err(|e| Error::msg(format!("cannot read object: {e}")))?
            .peel_to_tree()
            .map_err(|e| Error::msg(format!("cannot read tree: {e}")))
    }

    pub fn blob(&self, id: gix::ObjectId) -> Result<Blob> {
        let obj = self
            .repo
            .find_object(id)
            .map_err(|e| Error::msg(format!("cannot read blob: {e}")))?;
        Ok(Blob {
            data: obj.data.clone(),
            id,
        })
    }

    pub fn blob_at(&self, commit: gix::ObjectId, path: &str) -> Result<Option<Blob>> {
        match self.tree(commit)?.lookup_entry_by_path(path) {
            Ok(Some(entry)) => Ok(Some(self.blob(entry.object_id())?)),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::msg(format!("cannot look up {path}: {e}"))),
        }
    }

    /// A file as it sits in the working tree. It has no id in the object
    /// database yet, so we compute the one git would give it on commit.
    pub fn worktree_blob(&self, path: &str) -> Result<Option<Blob>> {
        let Some(dir) = self.workdir() else {
            return Ok(None);
        };
        let Ok(data) = std::fs::read(dir.join(path)) else {
            return Ok(None);
        };
        let id = gix::objs::compute_hash(self.repo.object_hash(), gix::object::Kind::Blob, &data)
            .map_err(|e| Error::msg(format!("cannot hash working tree file: {e}")))?;
        Ok(Some(Blob { data, id }))
    }

    // ---- differences ----------------------------------------------------

    /// The diff of one file, as git would compute it.
    ///
    /// The verdict of *whether* to diff is git's: a file whose content is
    /// binary, or marked `-diff` in `.gitattributes`, comes back as
    /// [`Diffed::Untouchable`] rather than being decoded into nonsense. So is
    /// the choice of algorithm, which follows `diff.algorithm` — the reviewer
    /// should see the hunks the author saw.
    pub fn diff(&self, path: &str, old: Side<'_>, new: Side<'_>) -> Result<Diffed> {
        use gix::diff::blob::ResourceKind;
        use gix::diff::blob::pipeline::{Mode, WorktreeRoots};
        use gix::diff::blob::platform::prepare_diff::Operation;

        let roots = WorktreeRoots {
            old_root: matches!(old, Side::Worktree).then(|| self.workdir_owned()),
            new_root: matches!(new, Side::Worktree).then(|| self.workdir_owned()),
        };
        let mut platform = self
            .repo
            .diff_resource_cache(Mode::ToGit, roots)
            .map_err(|e| Error::msg(format!("cannot prepare diff: {e}")))?;

        for (side, kind) in [
            (old, ResourceKind::OldOrSource),
            (new, ResourceKind::NewOrDestination),
        ] {
            platform
                .set_resource(
                    side.id(self.repo.object_hash()),
                    gix::object::tree::EntryKind::Blob,
                    path.into(),
                    kind,
                    &self.repo.objects,
                )
                .map_err(|e| Error::msg(format!("cannot read {path} for diff: {e}")))?;
        }

        let outcome = platform
            .prepare_diff()
            .map_err(|e| Error::msg(format!("cannot diff {path}: {e}")))?;

        let algorithm = match outcome.operation {
            Operation::InternalDiff { algorithm } => algorithm,
            // An external diff driver produces text for a human to read, not
            // hunks to render, and a binary file has no lines at all.
            Operation::ExternalCommand { .. } | Operation::SourceOrDestinationIsBinary => {
                return Ok(Diffed::Untouchable);
            }
        };

        Ok(Diffed::Text(super::text_diff::hunks(
            algorithm,
            &outcome.interned_input(),
        )))
    }

    fn workdir_owned(&self) -> std::path::PathBuf {
        self.workdir().unwrap_or(Path::new(".")).to_path_buf()
    }

    /// What turning `base` into `head` would take, renames included.
    pub fn tree_changes(
        &self,
        base: gix::ObjectId,
        head: gix::ObjectId,
    ) -> Result<Vec<gix::object::tree::diff::ChangeDetached>> {
        let base_tree = self.tree(base)?;
        let head_tree = self.tree(head)?;

        // Rewrite tracking is asked for explicitly rather than left to the
        // repo's configuration: whether a move is reported as a move should not
        // depend on whose machine farol is running on.
        let options =
            gix::diff::Options::default().with_rewrites(Some(gix::diff::Rewrites::default()));

        self.repo
            .diff_tree_to_tree(&base_tree, &head_tree, options)
            .map_err(|e| Error::msg(format!("cannot diff trees: {e}")))
    }

    /// Tracked files that differ from `HEAD` in the working tree, staged or
    /// not. This is git's own status, which decides from the index rather than
    /// by reading every tracked file — the difference between touching a
    /// handful of files and stat-ing the entire checkout.
    ///
    /// Untracked files are left out: farol reviews a branch, and pulling in
    /// scratch files would make the scope unpredictable.
    pub fn worktree_changes(&self) -> Result<Vec<String>> {
        if self.workdir().is_none() {
            return Ok(Vec::new());
        }

        let status = self
            .repo
            .status(gix::progress::Discard)
            .map_err(|e| Error::msg(format!("cannot read status: {e}")))?
            .into_iter(None)
            .map_err(|e| Error::msg(format!("cannot read status: {e}")))?;

        let mut out = Vec::new();
        for item in status {
            let item = item.map_err(|e| Error::msg(format!("cannot read status: {e}")))?;
            let path = match &item {
                gix::status::Item::TreeIndex(change) => change.location().to_string(),
                gix::status::Item::IndexWorktree(change) => match change {
                    gix::status::index_worktree::Item::Modification { rela_path, .. } => {
                        rela_path.to_string()
                    }
                    gix::status::index_worktree::Item::Rewrite { dirwalk_entry, .. } => {
                        dirwalk_entry.rela_path.to_string()
                    }
                    gix::status::index_worktree::Item::DirectoryContents { .. } => continue,
                },
            };
            out.push(path);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(content: &str) -> Blob {
        let data = content.as_bytes().to_vec();
        let id =
            gix::objs::compute_hash(gix::hash::Kind::Sha1, gix::object::Kind::Blob, &data).unwrap();
        Blob { data, id }
    }

    #[test]
    fn a_blob_id_is_the_one_git_itself_would_give() {
        // Viewed state is keyed on this, and `git hash-object` is the contract.
        // The value below is what git prints for a file containing "hello\n".
        assert_eq!(
            blob("hello\n").hash(),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
    }

    #[test]
    fn identical_content_at_two_paths_has_one_id() {
        // What lets a rename be spotted, and what keeps a file marked as read
        // after it moves.
        assert_eq!(blob("same\n").id, blob("same\n").id);
        assert_ne!(blob("same\n").id, blob("other\n").id);
    }
}
