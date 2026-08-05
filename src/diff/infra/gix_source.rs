use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::diff::domain::{
    DiffSource, FileChange, FileDiff, FileStatus, Hunk, Line, LineKind, Scope,
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
    /// path -> blob content on each side, for the review window.
    base_blobs: BTreeMap<String, Vec<u8>>,
    head_blobs: BTreeMap<String, Vec<u8>>,
}

impl GixSource {
    /// The branch comes from the workspace rather than being rediscovered
    /// here: one place decides what branch we are on, and it already refused a
    /// detached HEAD.
    pub fn open(repo: gix::Repository, current: &str, req: &ScopeRequest) -> Result<Self> {
        let current = current.to_string();

        let base_ref = match &req.base {
            Some(b) => b.clone(),
            None => default_base(&repo)?,
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

        let head_id = resolve(&repo, &head_ref)?;
        let base_tip = resolve(&repo, &base_ref)?;
        let base_id = if req.direct {
            base_tip
        } else {
            merge_base(&repo, base_tip, head_id)?
        };

        let base_blobs = read_tree(&repo, base_id)?;
        let mut head_blobs = read_tree(&repo, head_id)?;

        if req.dirty {
            overlay_worktree(&repo, &mut head_blobs)?;
        }

        let files = changed_files(&base_blobs, &head_blobs);

        let scope = Scope {
            branch: current,
            base_ref,
            head_ref,
            base_sha: base_id.to_hex().to_string(),
            head_sha: head_id.to_hex().to_string(),
            merge_base: !req.direct,
            dirty: req.dirty,
            files,
        };

        Ok(Self {
            repo: repo.into_sync(),
            scope,
            base_blobs,
            head_blobs,
        })
    }

    /// A repository handle for this thread. Cheap — it shares the object
    /// database and only rebuilds the local caches.
    pub fn repo(&self) -> gix::Repository {
        self.repo.to_thread_local()
    }
}

impl DiffSource for GixSource {
    fn scope(&self) -> Result<&Scope> {
        Ok(&self.scope)
    }

    fn file_diff(&self, path: &str) -> Result<FileDiff> {
        let change = self
            .scope
            .files
            .iter()
            .find(|f| f.path == path)
            .ok_or_else(|| self.scope.reject(path))?;

        let old_key = change.old_path.clone().unwrap_or_else(|| path.to_string());
        let old = self
            .base_blobs
            .get(&old_key)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let new = self.head_blobs.get(path).map(Vec::as_slice).unwrap_or(&[]);

        Ok(build_file_diff(
            path,
            change.old_path.clone(),
            change.status,
            old,
            new,
        ))
    }

    fn file_diff_between(&self, from: &str, to: &str, path: &str) -> Result<Option<FileDiff>> {
        let repo = self.repo();
        let from_id = resolve(&repo, from)?;
        let old = blob_at(&repo, from_id, path)?;

        let new = if to == crate::shared::WORKING {
            std::fs::read(
                repo.workdir()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(path),
            )
            .ok()
        } else {
            let to_id = resolve(&repo, to)?;
            blob_at(&repo, to_id, path)?
        };

        match (old, new) {
            (None, None) => Ok(None),
            (a, b) => {
                let a = a.unwrap_or_default();
                let b = b.unwrap_or_default();
                if a == b {
                    return Ok(None);
                }
                Ok(Some(build_file_diff(
                    path,
                    None,
                    FileStatus::Modified,
                    &a,
                    &b,
                )))
            }
        }
    }

    fn file_line_count(&self, path: &str) -> Result<u32> {
        let content = self
            .head_blobs
            .get(path)
            .ok_or_else(|| self.scope.reject(path))?;
        Ok(String::from_utf8_lossy(content).lines().count() as u32)
    }

    fn head_sha(&self) -> Result<String> {
        Ok(self.scope.head_sha.clone())
    }

    fn commits_ahead_of(&self, sha: &str) -> Result<u32> {
        if sha == crate::shared::WORKING {
            return Ok(0);
        }
        let repo = self.repo();
        let target = resolve(&repo, sha)?;
        let head = resolve(&repo, &self.scope.head_sha)?;
        if target == head {
            return Ok(0);
        }
        let walk = repo
            .rev_walk([head])
            .all()
            .map_err(|e| Error::msg(format!("cannot walk history: {e}")))?;
        let mut n = 0u32;
        for info in walk {
            let info = info.map_err(|e| Error::msg(format!("cannot walk history: {e}")))?;
            if info.id == target {
                return Ok(n);
            }
            n += 1;
            if n > 10_000 {
                break;
            }
        }
        Ok(n)
    }

    fn is_ancestor(&self, sha: &str) -> Result<bool> {
        if sha == crate::shared::WORKING {
            return Ok(false);
        }
        let repo = self.repo();
        let Ok(target) = resolve(&repo, sha) else {
            return Ok(false);
        };
        let head = resolve(&repo, &self.scope.head_sha)?;
        if target == head {
            return Ok(true);
        }
        let walk = repo
            .rev_walk([head])
            .all()
            .map_err(|e| Error::msg(format!("cannot walk history: {e}")))?;
        for info in walk {
            let info = info.map_err(|e| Error::msg(format!("cannot walk history: {e}")))?;
            if info.id == target {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

// ---- git plumbing ------------------------------------------------------

/// `main`, then `master`. Repos that predate the rename are still common enough
/// that failing on the first try would be a daily annoyance.
fn default_base(repo: &gix::Repository) -> Result<String> {
    for candidate in ["main", "master"] {
        if resolve(repo, candidate).is_ok() {
            return Ok(candidate.to_string());
        }
    }
    Err(Error::NoBaseBranch)
}

fn resolve(repo: &gix::Repository, rev: &str) -> Result<gix::ObjectId> {
    repo.rev_parse_single(rev)
        .map(|id| id.detach())
        .map_err(|e| Error::msg(format!("cannot resolve '{rev}': {e}")))
}

fn merge_base(repo: &gix::Repository, a: gix::ObjectId, b: gix::ObjectId) -> Result<gix::ObjectId> {
    repo.merge_base(a, b)
        .map(|id| id.detach())
        .map_err(|e| Error::msg(format!("cannot find merge base: {e}")))
}

/// Flatten a commit's tree into path -> content.
fn read_tree(repo: &gix::Repository, commit: gix::ObjectId) -> Result<BTreeMap<String, Vec<u8>>> {
    let tree = repo
        .find_object(commit)
        .map_err(|e| Error::msg(format!("cannot read commit: {e}")))?
        .peel_to_tree()
        .map_err(|e| Error::msg(format!("cannot read tree: {e}")))?;

    let mut recorder = gix::traverse::tree::Recorder::default();
    tree.traverse()
        .breadthfirst(&mut recorder)
        .map_err(|e| Error::msg(format!("cannot walk tree: {e}")))?;

    let mut out = BTreeMap::new();
    for entry in recorder.records {
        if !entry.mode.is_blob() {
            continue;
        }
        let path = entry.filepath.to_string();
        let data = repo
            .find_object(entry.oid)
            .map_err(|e| Error::msg(format!("cannot read blob for {path}: {e}")))?
            .data
            .clone();
        out.insert(path, data);
    }
    Ok(out)
}

fn blob_at(repo: &gix::Repository, commit: gix::ObjectId, path: &str) -> Result<Option<Vec<u8>>> {
    let tree = repo
        .find_object(commit)
        .map_err(|e| Error::msg(format!("cannot read commit: {e}")))?
        .peel_to_tree()
        .map_err(|e| Error::msg(format!("cannot read tree: {e}")))?;

    match tree.lookup_entry_by_path(path) {
        Ok(Some(entry)) => {
            let obj = repo
                .find_object(entry.object_id())
                .map_err(|e| Error::msg(format!("cannot read blob: {e}")))?;
            Ok(Some(obj.data.clone()))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(Error::msg(format!("cannot look up {path}: {e}"))),
    }
}

/// Replace committed contents with what is on disk, so uncommitted work shows
/// up in the same review window.
fn overlay_worktree(
    repo: &gix::Repository,
    head_blobs: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let Some(workdir) = repo.workdir() else {
        return Ok(());
    };

    let mut gone = Vec::new();
    for (path, committed) in head_blobs.iter_mut() {
        let full = workdir.join(path);
        match std::fs::read(&full) {
            Ok(disk) => {
                if &disk != committed {
                    *committed = disk;
                }
            }
            Err(_) => gone.push(path.clone()),
        }
    }
    for path in gone {
        head_blobs.remove(&path);
    }

    // Untracked files are not swept in: farol reviews a branch, and pulling in
    // scratch files would make the scope unpredictable.
    Ok(())
}

fn changed_files(
    base: &BTreeMap<String, Vec<u8>>,
    head: &BTreeMap<String, Vec<u8>>,
) -> Vec<FileChange> {
    let mut out = Vec::new();
    let mut removed: Vec<&String> = Vec::new();

    for (path, new) in head {
        match base.get(path) {
            Some(old) if old == new => {}
            Some(old) => out.push(counted(path, None, FileStatus::Modified, old, new)),
            None => out.push(counted(path, None, FileStatus::Added, &[], new)),
        }
    }
    for path in base.keys() {
        if !head.contains_key(path) {
            removed.push(path);
        }
    }

    // A file that vanished from one path and appeared byte-identical at another
    // is a rename, and saying so spares the reviewer reading it twice.
    for path in removed {
        let old = &base[path];
        let renamed_to = out.iter().position(|c| {
            c.status == FileStatus::Added && head.get(&c.path).map(|n| n == old).unwrap_or(false)
        });
        match renamed_to {
            Some(idx) => {
                out[idx].status = FileStatus::Renamed;
                out[idx].old_path = Some(path.clone());
                out[idx].additions = 0;
                out[idx].deletions = 0;
            }
            None => out.push(counted(path, None, FileStatus::Deleted, old, &[])),
        }
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn counted(
    path: &str,
    old_path: Option<String>,
    status: FileStatus,
    old: &[u8],
    new: &[u8],
) -> FileChange {
    let (additions, deletions) = count_changes(old, new);
    FileChange {
        path: path.to_string(),
        old_path,
        status,
        additions,
        deletions,
    }
}

fn count_changes(old: &[u8], new: &[u8]) -> (u32, u32) {
    let old = String::from_utf8_lossy(old);
    let new = String::from_utf8_lossy(new);
    let diff = similar::TextDiff::from_lines(old.as_ref(), new.as_ref());
    let mut adds = 0;
    let mut dels = 0;
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Insert => adds += 1,
            similar::ChangeTag::Delete => dels += 1,
            similar::ChangeTag::Equal => {}
        }
    }
    (adds, dels)
}

// ---- diff building -----------------------------------------------------

const CONTEXT: usize = 3;

pub fn build_file_diff(
    path: &str,
    old_path: Option<String>,
    status: FileStatus,
    old: &[u8],
    new: &[u8],
) -> FileDiff {
    let old_text = String::from_utf8_lossy(old).into_owned();
    let new_text = String::from_utf8_lossy(new).into_owned();
    let diff = similar::TextDiff::from_lines(&old_text, &new_text);

    let mut hunks = Vec::new();
    let mut additions = 0u32;
    let mut deletions = 0u32;

    for group in diff.grouped_ops(CONTEXT) {
        let mut lines = Vec::new();
        let mut old_start = 0u32;
        let mut new_start = 0u32;
        let mut old_lines = 0u32;
        let mut new_lines = 0u32;
        let mut first = true;

        for op in &group {
            for change in diff.iter_changes(op) {
                let old_number = change.old_index().map(|i| i as u32 + 1);
                let new_number = change.new_index().map(|i| i as u32 + 1);
                if first {
                    old_start = old_number.unwrap_or(1);
                    new_start = new_number.unwrap_or(1);
                    first = false;
                }
                let kind = match change.tag() {
                    similar::ChangeTag::Insert => {
                        additions += 1;
                        new_lines += 1;
                        LineKind::Added
                    }
                    similar::ChangeTag::Delete => {
                        deletions += 1;
                        old_lines += 1;
                        LineKind::Removed
                    }
                    similar::ChangeTag::Equal => {
                        old_lines += 1;
                        new_lines += 1;
                        LineKind::Context
                    }
                };
                lines.push(Line {
                    kind,
                    old_number,
                    new_number,
                    content: change.value().trim_end_matches('\n').to_string(),
                });
            }
        }

        hunks.push(Hunk {
            old_start,
            old_lines,
            new_start,
            new_lines,
            lines,
        });
    }

    FileDiff {
        path: path.to_string(),
        old_path,
        status,
        hunks,
        additions,
        deletions,
        new_content_hash: hash(new),
    }
}

pub fn hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
        entries
            .iter()
            .map(|(p, c)| (p.to_string(), c.as_bytes().to_vec()))
            .collect()
    }

    fn find<'a>(changes: &'a [FileChange], path: &str) -> &'a FileChange {
        changes
            .iter()
            .find(|c| c.path == path)
            .unwrap_or_else(|| panic!("{path} missing from {:?}", changes))
    }

    #[test]
    fn a_file_only_on_the_new_side_is_added() {
        let changes = changed_files(&tree(&[]), &tree(&[("a.rs", "hello\n")]));
        assert_eq!(find(&changes, "a.rs").status, FileStatus::Added);
        assert_eq!(find(&changes, "a.rs").additions, 1);
    }

    #[test]
    fn a_file_only_on_the_old_side_is_deleted() {
        let changes = changed_files(&tree(&[("a.rs", "hello\n")]), &tree(&[]));
        assert_eq!(find(&changes, "a.rs").status, FileStatus::Deleted);
        assert_eq!(find(&changes, "a.rs").deletions, 1);
    }

    #[test]
    fn an_unchanged_file_does_not_appear_at_all() {
        let same = tree(&[("a.rs", "hello\n")]);
        assert!(changed_files(&same, &same).is_empty());
    }

    #[test]
    fn a_file_that_moved_with_identical_content_is_a_rename() {
        // Reporting this as an add plus a delete would make the reviewer read
        // the whole file twice for a change that is not there.
        let changes = changed_files(
            &tree(&[("old/a.rs", "hello\nworld\n")]),
            &tree(&[("new/a.rs", "hello\nworld\n")]),
        );

        assert_eq!(
            changes.len(),
            1,
            "a rename is one entry, not two: {changes:?}"
        );
        let renamed = find(&changes, "new/a.rs");
        assert_eq!(renamed.status, FileStatus::Renamed);
        assert_eq!(renamed.old_path.as_deref(), Some("old/a.rs"));
        assert_eq!(
            (renamed.additions, renamed.deletions),
            (0, 0),
            "nothing changed inside it"
        );
    }

    #[test]
    fn a_moved_file_whose_content_also_changed_is_not_treated_as_a_rename() {
        // The content moved *and* changed, so the reviewer does have to read it.
        let changes = changed_files(
            &tree(&[("old/a.rs", "hello\n")]),
            &tree(&[("new/a.rs", "hello\nplus a line\n")]),
        );
        assert_eq!(changes.len(), 2);
        assert_eq!(find(&changes, "new/a.rs").status, FileStatus::Added);
        assert_eq!(find(&changes, "old/a.rs").status, FileStatus::Deleted);
    }

    #[test]
    fn changes_come_back_in_path_order() {
        let changes = changed_files(
            &tree(&[]),
            &tree(&[("z.rs", "z\n"), ("a.rs", "a\n"), ("m.rs", "m\n")]),
        );
        let paths: Vec<&str> = changes.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, vec!["a.rs", "m.rs", "z.rs"]);
    }

    #[test]
    fn a_diff_carries_hunks_with_the_line_numbers_the_notes_will_use() {
        let old = "one\ntwo\nthree\nfour\nfive\n";
        let new = "one\ntwo\nCHANGED\nfour\nfive\n";
        let diff = build_file_diff(
            "a.rs",
            None,
            FileStatus::Modified,
            old.as_bytes(),
            new.as_bytes(),
        );

        assert_eq!((diff.additions, diff.deletions), (1, 1));
        assert_eq!(diff.hunks.len(), 1);

        let changed: Vec<_> = diff.hunks[0]
            .lines
            .iter()
            .filter(|l| l.kind != LineKind::Context)
            .collect();
        assert_eq!(changed.len(), 2);
        assert_eq!(changed[0].content, "three");
        assert_eq!(changed[0].old_number, Some(3));
        assert_eq!(changed[1].content, "CHANGED");
        assert_eq!(changed[1].new_number, Some(3));
    }

    #[test]
    fn distant_edits_land_in_separate_hunks() {
        // Line notes shift per hunk, so this split is not cosmetic.
        let old: String = (1..=40).map(|i| format!("line {i}\n")).collect();
        let new = old
            .replace("line 2\n", "CHANGED 2\n")
            .replace("line 38\n", "CHANGED 38\n");
        let diff = build_file_diff(
            "a.rs",
            None,
            FileStatus::Modified,
            old.as_bytes(),
            new.as_bytes(),
        );
        assert_eq!(diff.hunks.len(), 2);
    }

    #[test]
    fn the_content_hash_follows_the_new_side_only() {
        // Viewed-state invalidation keys on this; if it moved with the old side
        // a rebase would reopen files nobody touched.
        let a = build_file_diff("a.rs", None, FileStatus::Modified, b"one\n", b"two\n");
        let b = build_file_diff("a.rs", None, FileStatus::Modified, b"other\n", b"two\n");
        assert_eq!(a.new_content_hash, b.new_content_hash);

        let c = build_file_diff("a.rs", None, FileStatus::Modified, b"one\n", b"three\n");
        assert_ne!(a.new_content_hash, c.new_content_hash);
    }

    #[test]
    fn a_near_miss_path_is_offered_back_best_match_first() {
        let scope = Scope {
            branch: "b".into(),
            base_ref: "main".into(),
            head_ref: "b".into(),
            base_sha: "x".into(),
            head_sha: "y".into(),
            merge_base: true,
            dirty: false,
            files: ["services/db.go", "io/db_test.go", "unrelated.rs"]
                .iter()
                .map(|p| counted(p, None, FileStatus::Modified, b"", b""))
                .collect(),
        };

        let hits = scope.similar_paths("service/db.go");
        assert_eq!(hits.first().map(String::as_str), Some("services/db.go"));
        assert!(scope.similar_paths("nothing_like_it.py").is_empty());
    }
}
