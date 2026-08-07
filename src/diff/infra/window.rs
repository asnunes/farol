//! What is under review, read from git.
//!
//! Only the files in the window are ever materialised. Asking git for the
//! difference between two trees skips identical subtrees whole, so a branch
//! touching ten files does not pay for the rest of the checkout — which, in a
//! repo with a vendor directory, was a gigabyte held for as long as the server
//! ran.

use std::collections::{BTreeMap, BTreeSet};

use super::blob::{Blob, Diffed, Side};
use super::git::Git;
use crate::diff::domain::{FileChange, FileStatus};
use crate::shared::error::Result;

/// The review window with both sides read in.
pub(super) struct Window {
    pub files: Vec<FileChange>,
    pub base_blobs: BTreeMap<String, Blob>,
    pub head_blobs: BTreeMap<String, Blob>,
    /// Paths whose new side is uncommitted work, so it has no object to point
    /// at and has to be read from disk again when the diff is asked for.
    pub from_worktree: BTreeSet<String>,
}

impl Window {
    pub fn of(git: &Git, base: gix::ObjectId, head: gix::ObjectId, dirty: bool) -> Result<Self> {
        let mut named = Self::named(git, base, head)?;
        if dirty {
            Self::add_worktree(git, &mut named)?;
        }
        Self::read(git, base, named)
    }

    /// Name both sides of every changed file, without reading any of them.
    fn named(
        git: &Git,
        base: gix::ObjectId,
        head: gix::ObjectId,
    ) -> Result<BTreeMap<String, Sides>> {
        use gix::object::tree::diff::ChangeDetached as C;

        let mut out: BTreeMap<String, Sides> = BTreeMap::new();
        for change in git.tree_changes(base, head)? {
            match change {
                C::Addition {
                    location,
                    entry_mode,
                    id,
                    ..
                } if entry_mode.is_blob() => {
                    out.entry(location.to_string()).or_default().new_id = Some(id);
                }
                C::Deletion {
                    location,
                    entry_mode,
                    id,
                    ..
                } if entry_mode.is_blob() => {
                    out.entry(location.to_string()).or_default().old_id = Some(id);
                }
                C::Modification {
                    location,
                    entry_mode,
                    previous_id,
                    id,
                    ..
                } if entry_mode.is_blob() => {
                    let sides = out.entry(location.to_string()).or_default();
                    sides.old_id = Some(previous_id);
                    sides.new_id = Some(id);
                }
                C::Rewrite {
                    location,
                    source_location,
                    source_id,
                    id,
                    entry_mode,
                    ..
                } if entry_mode.is_blob() => {
                    let sides = out.entry(location.to_string()).or_default();
                    sides.old_path = Some(source_location.to_string());
                    sides.old_id = Some(source_id);
                    sides.new_id = Some(id);
                }
                // Trees and submodules are not files anyone reviews line by line.
                _ => {}
            }
        }
        Ok(out)
    }

    /// Fold uncommitted work into the window.
    fn add_worktree(git: &Git, named: &mut BTreeMap<String, Sides>) -> Result<()> {
        for path in git.worktree_changes()? {
            let sides = named.entry(path).or_default();
            sides.from_worktree = true;
            sides.new_id = None;
        }
        Ok(())
    }

    /// Read the bytes for the window, and only for the window.
    fn read(git: &Git, base: gix::ObjectId, named: BTreeMap<String, Sides>) -> Result<Self> {
        let mut files = Vec::new();
        let mut base_blobs = BTreeMap::new();
        let mut head_blobs = BTreeMap::new();
        let mut from_worktree = BTreeSet::new();

        for (path, sides) in named {
            let old_key = sides.old_path.clone().unwrap_or_else(|| path.clone());

            let old = match sides.old_id {
                Some(id) => Some(git.blob(id)?),
                // A path git only knows about from the worktree still has a
                // base side, and the tree is where to find it.
                None if sides.from_worktree => git.blob_at(base, &old_key)?,
                None => None,
            };

            let new = match sides.new_id {
                Some(id) => Some(git.blob(id)?),
                None if sides.from_worktree => git.worktree_blob(&path)?,
                None => None,
            };

            // A file whose uncommitted edits happen to restore the base is not
            // a change, whatever status said about it. A rename is exempt:
            // moving a file without touching it leaves both sides the same blob
            // on purpose, and that move is still something to report.
            if sides.old_path.is_none() && old.as_ref().map(|b| b.id) == new.as_ref().map(|b| b.id)
            {
                continue;
            }

            let status = match (&sides.old_path, &old, &new) {
                (Some(_), _, _) => FileStatus::Renamed,
                (None, None, Some(_)) => FileStatus::Added,
                (None, Some(_), None) => FileStatus::Deleted,
                _ => FileStatus::Modified,
            };

            // The churn in the file header is git's count, which means a
            // binary file reports nothing rather than a made-up number of
            // lines.
            let (additions, deletions) = match git.diff(
                &path,
                old.as_ref().map(Side::Object).unwrap_or(Side::Absent),
                Self::new_side(new.as_ref(), sides.from_worktree),
            )? {
                Diffed::Untouchable => (0, 0),
                Diffed::Text(h) => (h.additions, h.deletions),
            };

            files.push(FileChange {
                path: path.clone(),
                old_path: sides.old_path.clone(),
                status,
                additions,
                deletions,
            });

            if let Some(blob) = old {
                base_blobs.insert(old_key, blob);
            }
            if let Some(blob) = new {
                if sides.from_worktree {
                    from_worktree.insert(path.clone());
                }
                head_blobs.insert(path, blob);
            }
        }

        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self {
            files,
            base_blobs,
            head_blobs,
            from_worktree,
        })
    }

    fn new_side(blob: Option<&Blob>, from_worktree: bool) -> Side<'_> {
        match (blob, from_worktree) {
            (None, _) => Side::Absent,
            (Some(_), true) => Side::Worktree,
            (Some(blob), false) => Side::Object(blob),
        }
    }
}

/// One file in the window, named by both sides before either is read.
///
/// Ids, not bytes: this is what comes back from a tree diff, and holding it
/// this way means a file is only fetched from the object database once it is
/// known to be part of the review.
#[derive(Default)]
struct Sides {
    /// Where the file came from, when git recognised a rename.
    old_path: Option<String>,
    old_id: Option<gix::ObjectId>,
    new_id: Option<gix::ObjectId>,
    /// Set when the new side is uncommitted work, which has no id in the object
    /// database until it is read off disk.
    from_worktree: bool,
}
