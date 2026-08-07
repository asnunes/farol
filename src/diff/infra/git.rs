//! The repository, and the questions farol asks it.
//!
//! Everything that speaks `gix` lives here, so the rest of the layer works in
//! object ids and blobs and never learns which library answers. The repository
//! is held, not passed: a caller gets a `Git` and asks it things, rather than
//! threading a `&Repository` through every function it calls.

use std::path::Path;

use crate::shared::error::{Error, Result};

use super::blob::{Blob, Diffed, Side};

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
    use crate::diff::domain::LineKind;
    use crate::diff::infra::fixture::Fixture;

    // ---- revisions ------------------------------------------------------

    #[test]
    fn the_base_branch_falls_back_to_master_and_then_gives_up() {
        let f = Fixture::new();
        assert_eq!(f.open().default_base().unwrap(), "main");

        f.git(&["branch", "-m", "main", "master"]);
        assert_eq!(f.open().default_base().unwrap(), "master");

        f.git(&["branch", "-m", "master", "trunk"]);
        assert!(
            matches!(f.open().default_base(), Err(Error::NoBaseBranch)),
            "neither name exists, and guessing a third would be worse than asking"
        );
    }

    #[test]
    fn an_unknown_revision_is_named_in_the_error() {
        // The author typed it; the error is useless if it does not say what.
        let err = Fixture::new().open().resolve("no-such-branch").unwrap_err();
        assert!(err.to_string().contains("no-such-branch"), "{err}");
    }

    #[test]
    fn a_commit_is_not_behind_itself() {
        let f = Fixture::new();
        let head = f.sha("HEAD");
        assert_eq!(f.open().commits_between(head, head).unwrap(), 0);
    }

    #[test]
    fn distance_counts_only_what_came_after() {
        let f = Fixture::new();
        let from = f.sha("HEAD");
        for i in 1..=3 {
            f.write("a.txt", &format!("{i}\n"));
            f.commit("more");
        }
        assert_eq!(f.open().commits_between(from, f.sha("HEAD")).unwrap(), 3);
    }

    #[test]
    fn a_commit_is_its_own_ancestor() {
        // What makes the map of the current commit count as current.
        let f = Fixture::new();
        let head = f.sha("HEAD");
        assert!(f.open().is_ancestor(head, head));
    }

    #[test]
    fn a_commit_that_was_thrown_away_is_not_an_ancestor() {
        let f = Fixture::new();
        f.write("a.txt", "one\n");
        f.commit("will be discarded");
        let discarded = f.sha("HEAD");
        f.git(&["reset", "-q", "--hard", "HEAD~1"]);

        assert!(
            !f.open().is_ancestor(discarded, f.sha("HEAD")),
            "its map describes code that is no longer on this branch"
        );
    }

    #[test]
    fn a_history_with_nothing_in_common_is_not_an_ancestor() {
        // Unrelated histories have no merge base at all. The answer is "no",
        // not an error — this is the branch that swallows one.
        let f = Fixture::new();
        let main = f.sha("HEAD");
        f.git(&["checkout", "-q", "--orphan", "other"]);
        f.write("only-here.txt", "unrelated\n");
        f.commit("a history of its own");

        assert!(!f.open().is_ancestor(main, f.sha("HEAD")));
    }

    // ---- objects --------------------------------------------------------

    #[test]
    fn a_blob_read_from_a_tree_carries_the_id_git_gives_it() {
        let f = Fixture::new();
        let expected = f.git(&["rev-parse", "HEAD:README.md"]);

        let blob = f
            .open()
            .blob_at(f.sha("HEAD"), "README.md")
            .unwrap()
            .unwrap();

        assert_eq!(blob.hash(), expected, "the contract is `git hash-object`");
        assert_eq!(blob.data, b"start\n");
    }

    #[test]
    fn a_path_that_is_not_in_the_tree_is_absent_rather_than_an_error() {
        // Deriving asks for the old side of files that may not have existed yet.
        let f = Fixture::new();
        assert!(
            f.open()
                .blob_at(f.sha("HEAD"), "never-existed.rs")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_worktree_file_gets_the_id_it_will_have_once_committed() {
        // This is what lets a file marked as read under --dirty stay read after
        // the commit: the same content keeps the same identity.
        let f = Fixture::new();
        f.write("draft.txt", "work in progress\n");
        let uncommitted = f.open().worktree_blob("draft.txt").unwrap().unwrap();

        f.commit("commit the draft");
        let committed = f.git(&["rev-parse", "HEAD:draft.txt"]);

        assert_eq!(uncommitted.hash(), committed);
    }

    #[test]
    fn a_file_that_is_not_on_disk_has_no_worktree_blob() {
        let f = Fixture::new();
        assert!(f.open().worktree_blob("deleted.txt").unwrap().is_none());
    }

    // ---- what changed ---------------------------------------------------

    #[test]
    fn a_move_comes_back_as_one_rewrite_and_not_a_pair() {
        let f = Fixture::new();
        let base = f.sha("HEAD");
        f.git(&["mv", "README.md", "docs.md"]);
        f.commit("rename");

        let changes = f.open().tree_changes(base, f.sha("HEAD")).unwrap();

        assert_eq!(changes.len(), 1, "{changes:?}");
        assert!(matches!(
            &changes[0],
            gix::object::tree::diff::ChangeDetached::Rewrite { .. }
        ));
    }

    #[test]
    fn staged_and_unstaged_work_both_count_as_worktree_changes() {
        let f = Fixture::new();
        f.write("staged.txt", "new\n");
        f.git(&["add", "staged.txt"]);
        f.write("README.md", "edited\n");
        f.write("untracked.txt", "scratch\n");

        let changed = f.open().worktree_changes().unwrap();

        assert!(changed.contains(&"staged.txt".to_string()), "{changed:?}");
        assert!(changed.contains(&"README.md".to_string()), "{changed:?}");
        assert!(
            !changed.contains(&"untracked.txt".to_string()),
            "scratch files would make the window unpredictable: {changed:?}"
        );
    }

    // ---- diffing --------------------------------------------------------

    #[test]
    fn a_binary_file_comes_back_untouchable_rather_than_decoded() {
        let f = Fixture::new();
        f.write("logo.png", "");
        std::fs::write(
            f.dir.path().join("logo.png"),
            (0u8..=255).cycle().take(4000).collect::<Vec<u8>>(),
        )
        .unwrap();
        f.commit("add a binary");

        let git = f.open();
        let blob = git.blob_at(f.sha("HEAD"), "logo.png").unwrap().unwrap();

        assert!(matches!(
            git.diff("logo.png", Side::Absent, Side::Object(&blob))
                .unwrap(),
            Diffed::Untouchable
        ));
    }

    #[test]
    fn a_file_the_repository_marked_not_diffable_is_left_alone() {
        let f = Fixture::new();
        f.write(".gitattributes", "generated.txt -diff\n");
        f.write("generated.txt", "one\ntwo\n");
        f.commit("add generated output");

        let git = f.open();
        let blob = git
            .blob_at(f.sha("HEAD"), "generated.txt")
            .unwrap()
            .unwrap();

        assert!(
            matches!(
                git.diff("generated.txt", Side::Absent, Side::Object(&blob))
                    .unwrap(),
                Diffed::Untouchable
            ),
            "the repository asked for this not to be read line by line"
        );
    }

    #[test]
    fn an_added_file_diffs_as_all_additions_against_nothing() {
        // `Side::Absent` has to mean "not there", not "an empty object" — a
        // fresh repository has never stored the empty blob.
        let f = Fixture::new();
        f.write("new.txt", "one\ntwo\n");
        f.commit("add");

        let git = f.open();
        let blob = git.blob_at(f.sha("HEAD"), "new.txt").unwrap().unwrap();

        let Diffed::Text(out) = git
            .diff("new.txt", Side::Absent, Side::Object(&blob))
            .unwrap()
        else {
            panic!("a text file should be diffable");
        };
        assert_eq!((out.additions, out.deletions), (2, 0));
        assert!(
            out.hunks[0].lines.iter().all(|l| l.kind == LineKind::Added),
            "there is no old side to draw context from"
        );
    }

    #[test]
    fn uncommitted_content_is_diffed_against_what_was_committed() {
        let f = Fixture::new();
        f.write("a.txt", "one\ntwo\n");
        f.commit("commit it");
        let committed = f.open().blob_at(f.sha("HEAD"), "a.txt").unwrap().unwrap();
        f.write("a.txt", "one\nCHANGED\n");

        let Diffed::Text(out) = f
            .open()
            .diff("a.txt", Side::Object(&committed), Side::Worktree)
            .unwrap()
        else {
            panic!("a text file should be diffable");
        };

        assert_eq!((out.additions, out.deletions), (1, 1));
        assert!(
            out.hunks[0]
                .lines
                .iter()
                .any(|l| l.kind == LineKind::Added && l.content == "CHANGED"),
            "the new side has to come off disk, not out of the object database"
        );
    }
}
