//! What is under review, read from git.
//!
//! Only the files in the window are ever materialised. Asking git for the
//! difference between two trees skips identical subtrees whole, so a branch
//! touching ten files does not pay for the rest of the checkout — which, in a
//! repo with a vendor directory, was a gigabyte held for as long as the server
//! ran.

use std::collections::BTreeMap;

use super::blob::{Blob, Diffed, Side};
use super::git::Git;
use crate::diff::domain::{FileChange, FileStatus};
use crate::error::Result;

/// The review window with both sides read in.
pub(super) struct Window {
    pub files: Vec<FileChange>,
    pub base_blobs: BTreeMap<String, Blob>,
    pub head_blobs: BTreeMap<String, Blob>,
}

impl Window {
    pub fn of(git: &Git, base: gix::ObjectId, head: gix::ObjectId) -> Result<Self> {
        let named = Self::named(git, base, head)?;
        Self::read(git, named)
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

    /// Read the bytes for the window, and only for the window.
    fn read(git: &Git, named: BTreeMap<String, Sides>) -> Result<Self> {
        let mut files = Vec::new();
        let mut base_blobs = BTreeMap::new();
        let mut head_blobs = BTreeMap::new();

        for (path, sides) in named {
            let old_key = sides.old_path.clone().unwrap_or_else(|| path.clone());

            let old = sides.old_id.map(|id| git.blob(id)).transpose()?;
            let new = sides.new_id.map(|id| git.blob(id)).transpose()?;

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
                new.as_ref().map(Side::Object).unwrap_or(Side::Absent),
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
                // The new side, or the old one for a file that was deleted:
                // either way it is the content the reviewer is being shown.
                content_hash: new
                    .as_ref()
                    .or(old.as_ref())
                    .map(Blob::hash)
                    .unwrap_or_default(),
            });

            if let Some(blob) = old {
                base_blobs.insert(old_key, blob);
            }
            if let Some(blob) = new {
                head_blobs.insert(path, blob);
            }
        }

        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self {
            files,
            base_blobs,
            head_blobs,
        })
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::infra::fixture::Fixture;

    fn numbered(lines: usize) -> String {
        (1..=lines).map(|i| format!("line {i}\n")).collect()
    }

    /// The window between `main` and the branch, as `open` would build it.
    fn window(f: &Fixture) -> Window {
        let git = f.open();
        let base = git.resolve("main").unwrap();
        let head = git.resolve("HEAD").unwrap();
        Window::of(&git, base, head).expect("the window should build")
    }

    fn paths(w: &Window) -> Vec<&str> {
        w.files.iter().map(|c| c.path.as_str()).collect()
    }

    #[test]
    fn only_what_the_branch_touched_is_in_the_window() {
        // The rest of the checkout is not read at all, which is the whole
        // reason this goes through a tree diff.
        let f = Fixture::new();
        f.write("untouched.rs", "stays\n");
        f.commit("a file the branch will not touch");
        f.on_branch("feature/x");
        f.write("changed.rs", &numbered(10));
        f.commit("change one file");

        let w = window(&f);

        assert_eq!(paths(&w), vec!["changed.rs"]);
        assert!(!w.head_blobs.contains_key("untouched.rs"));
        assert!(!w.base_blobs.contains_key("untouched.rs"));
    }

    #[test]
    fn both_sides_of_a_changed_file_are_read_in() {
        let f = Fixture::new();
        f.write("a.rs", &numbered(10));
        f.commit("add a");
        f.on_branch("feature/x");
        f.write("a.rs", &numbered(12));
        f.commit("extend a");

        let w = window(&f);

        assert_eq!(w.files[0].status, FileStatus::Modified);
        assert_eq!(w.base_blobs["a.rs"].data, numbered(10).into_bytes());
        assert_eq!(w.head_blobs["a.rs"].data, numbered(12).into_bytes());
    }

    #[test]
    fn an_added_file_has_no_base_side_and_a_deleted_one_has_no_head_side() {
        // Deliberately unalike, or git would pair them as a rename — which it
        // does, and which `a_rename_keeps_the_old_side...` covers.
        let f = Fixture::new();
        f.write("goes.rs", "the one that leaves\n");
        f.commit("add the one that will go");
        f.on_branch("feature/x");
        f.git(&["rm", "-q", "goes.rs"]);
        f.write("arrives.rs", &numbered(30));
        f.commit("swap them");

        let w = window(&f);

        assert!(!w.base_blobs.contains_key("arrives.rs"));
        assert!(!w.head_blobs.contains_key("goes.rs"));
        let statuses: Vec<_> = w.files.iter().map(|c| c.status).collect();
        assert_eq!(statuses, vec![FileStatus::Added, FileStatus::Deleted]);
    }

    #[test]
    fn a_rename_keeps_the_old_side_under_the_name_it_had() {
        // Reading the base under the new path would show the whole file as
        // added, which is the reading rename detection exists to prevent.
        let f = Fixture::new();
        f.write("old/a.rs", &numbered(40));
        f.commit("add a");
        f.on_branch("feature/x");
        std::fs::create_dir_all(f.dir.path().join("new")).unwrap();
        f.git(&["mv", "old/a.rs", "new/a.rs"]);
        f.commit("move it");

        let w = window(&f);

        assert_eq!(w.files[0].status, FileStatus::Renamed);
        assert_eq!(w.files[0].old_path.as_deref(), Some("old/a.rs"));
        assert!(
            w.base_blobs.contains_key("old/a.rs"),
            "filed under the old name"
        );
        assert!(w.head_blobs.contains_key("new/a.rs"));
    }

    #[test]
    fn the_window_is_listed_in_path_order() {
        let f = Fixture::new();
        f.on_branch("feature/x");
        for name in ["z.rs", "a.rs", "m.rs"] {
            f.write(name, "x\n");
        }
        f.commit("three files");

        assert_eq!(paths(&window(&f)), vec!["a.rs", "m.rs", "z.rs"]);
    }

    #[test]
    fn churn_comes_from_git_so_a_binary_file_reports_nothing() {
        let f = Fixture::new();
        f.on_branch("feature/x");
        std::fs::write(
            f.dir.path().join("logo.png"),
            (0u8..=255).cycle().take(4000).collect::<Vec<u8>>(),
        )
        .unwrap();
        f.commit("add a binary");

        let w = window(&f);

        assert_eq!((w.files[0].additions, w.files[0].deletions), (0, 0));
    }
}
