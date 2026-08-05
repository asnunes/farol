//! End-to-end exercise of the CLI against real repositories.
//!
//! These are the cases a fake `DiffSource` cannot vouch for: how git dirs
//! resolve inside a worktree, what a merge base actually returns once the base
//! branch has moved, and whether the commands refuse the things they promise to
//! refuse.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_farol");

struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    /// A repository with `main` holding one commit.
    fn new() -> Self {
        Self::with_default_branch("main")
    }

    fn with_default_branch(branch: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repo { dir };
        repo.git(&["init", "-q", "--initial-branch", branch]);
        repo.git(&["config", "user.email", "test@farol"]);
        repo.git(&["config", "user.name", "farol test"]);
        repo.write("README.md", "start\n");
        repo.git(&["add", "."]);
        repo.commit("initial");
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn git(&self, args: &[&str]) -> Output {
        self.git_in(self.path(), args)
    }

    fn git_in(&self, cwd: &Path, args: &[&str]) -> Output {
        let out = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("git should run");
        assert!(
            out.status.success(),
            "git {args:?} failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }

    fn write(&self, rel: &str, contents: &str) {
        self.write_in(self.path(), rel, contents)
    }

    fn write_in(&self, root: &Path, rel: &str, contents: &str) {
        let full = root.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, contents).unwrap();
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message, "--no-gpg-sign"]);
    }

    fn farol(&self, args: &[&str]) -> Output {
        self.farol_in(self.path(), args)
    }

    fn farol_in(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .current_dir(cwd)
            .output()
            .expect("farol should run")
    }

    /// Run and require success, returning stdout.
    fn ok(&self, args: &[&str]) -> String {
        let out = self.farol(args);
        assert!(
            out.status.success(),
            "farol {args:?} should have succeeded:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Run and require failure, returning stderr.
    fn fails(&self, args: &[&str]) -> String {
        let out = self.farol(args);
        assert!(
            !out.status.success(),
            "farol {args:?} should have failed but printed:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
        String::from_utf8_lossy(&out.stderr).into_owned()
    }

    /// Start the map for the current commit.
    fn derive(&self) -> String {
        self.ok(&["map", "derive"])
    }

    /// The block most tests need: one block holding the given files.
    fn core_block(&self, paths: &[&str]) {
        let mut args = vec!["block", "add", "core", "--title", "t", "--context", "c"];
        args.extend_from_slice(paths);
        self.ok(&args);
    }

    /// A branch off main with one file changed, ready to review.
    fn feature(&self) -> &Self {
        self.git(&["checkout", "-q", "-b", "feature/x"]);
        self.write("src/a.rs", &numbered(60));
        self.commit("add a");
        self
    }
}

/// A file with predictable line numbers, so ranges in tests mean something.
fn numbered(lines: usize) -> String {
    (1..=lines)
        .map(|i| format!("line {i}\n"))
        .collect::<String>()
}

// ---- scope resolution ---------------------------------------------------

#[test]
fn scope_lists_what_the_branch_changed() {
    let repo = Repo::new();
    repo.feature();
    let out = repo.ok(&["scope"]);
    assert!(out.contains("src/a.rs"), "{out}");
    assert!(out.contains("added"), "{out}");
}

#[test]
fn base_falls_back_to_master_when_there_is_no_main() {
    let repo = Repo::with_default_branch("master");
    repo.feature();
    let out = repo.ok(&["scope"]);
    assert!(out.contains("master..."), "{out}");
}

#[test]
fn merge_base_ignores_what_the_base_branch_did_afterwards() {
    let repo = Repo::new();
    repo.feature();

    // main moves on with a file of its own.
    repo.git(&["checkout", "-q", "main"]);
    repo.write("src/unrelated.rs", "not mine\n");
    repo.commit("unrelated work on main");
    repo.git(&["checkout", "-q", "feature/x"]);

    let merge_based = repo.ok(&["scope"]);
    assert!(merge_based.contains("src/a.rs"), "{merge_based}");
    assert!(
        !merge_based.contains("src/unrelated.rs"),
        "merge base must not drag in the base branch's own work:\n{merge_based}"
    );

    // Asking for a direct comparison shows it, as a deletion, which is exactly
    // why merge base is the default.
    let direct = repo.ok(&["scope", "--direct"]);
    assert!(direct.contains("src/unrelated.rs"), "{direct}");
}

#[test]
fn detached_head_is_refused_because_there_is_no_branch_to_key_state_on() {
    let repo = Repo::new();
    repo.feature();
    let sha = String::from_utf8_lossy(&repo.git(&["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    repo.git(&["checkout", "-q", &sha]);

    let err = repo.fails(&["scope"]);
    assert!(err.contains("detached"), "{err}");
}

#[test]
fn dirty_includes_uncommitted_work() {
    let repo = Repo::new();
    repo.feature();
    repo.write("src/b.rs", "brand new\n");

    let clean = repo.ok(&["scope"]);
    assert!(!clean.contains("src/b.rs"), "{clean}");

    // Untracked files stay out; the point of --dirty is work in progress on
    // files git already knows about.
    repo.git(&["add", "src/b.rs"]);
    repo.write("src/a.rs", &numbered(70));
    let dirty = repo.ok(&["scope", "--dirty"]);
    assert!(dirty.contains("uncommitted"), "{dirty}");
}

#[test]
fn dirty_is_refused_when_head_points_somewhere_you_are_not_standing() {
    let repo = Repo::new();
    repo.feature();
    repo.git(&["checkout", "-q", "main"]);

    // Uncommitted work belongs to the tree you are standing in. Asking for
    // feature/x's while sitting on main is a contradiction, and silently
    // ignoring the flag would let you believe you were reviewing your WIP.
    let err = repo.fails(&["serve", "main", "feature/x", "--dirty", "--no-open"]);
    assert!(err.contains("--dirty only works"), "{err}");
    assert!(err.contains("feature/x"), "{err}");
}

#[test]
fn a_renamed_file_is_reported_once_rather_than_as_an_add_and_a_delete() {
    let repo = Repo::new();
    repo.feature();
    // A file that exists on the base: renaming one the branch created would
    // just be an add at the final path, since the base never had it.
    std::fs::create_dir_all(repo.path().join("docs")).unwrap();
    repo.git(&["mv", "README.md", "docs/README.md"]);
    repo.commit("move it");

    let out = repo.ok(&["scope"]);
    assert!(out.contains("renamed"), "{out}");
    assert!(out.contains("docs/README.md (was README.md)"), "{out}");
    assert!(
        !out.contains("deleted"),
        "reporting a move as add plus delete makes the reviewer read the whole \
         file twice for a change that is not there:\n{out}"
    );
}

// ---- worktrees ----------------------------------------------------------

#[test]
fn state_lands_in_the_worktrees_own_git_dir() {
    let repo = Repo::new();
    repo.feature();
    repo.git(&["checkout", "-q", "main"]);

    let wt: PathBuf = repo.path().parent().unwrap().join("farol-wt-test");
    let _ = std::fs::remove_dir_all(&wt);
    repo.git(&["worktree", "add", "-q", wt.to_str().unwrap(), "feature/x"]);

    // Inside a worktree `.git` is a *file*. Anything that joined onto it would
    // fail here rather than create a directory.
    assert!(
        wt.join(".git").is_file(),
        ".git should be a file in a worktree"
    );

    let out = repo.farol_in(&wt, &["map", "derive"]);
    assert!(
        out.status.success(),
        "derive inside a worktree failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let private = repo
        .path()
        .join(".git/worktrees/farol-wt-test/farol/feature-x/maps");
    assert!(
        private.exists(),
        "expected state under the worktree's own git dir at {}",
        private.display()
    );

    repo.git(&["worktree", "remove", "--force", wt.to_str().unwrap()]);
}

// ---- the mapping cycle --------------------------------------------------

#[test]
fn a_full_cycle_ends_with_check_passing_and_show_reflecting_it() {
    let repo = Repo::new();
    repo.feature();

    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "The change itself",
        "--context",
        "Why this exists at all.",
    ]);
    repo.ok(&[
        "file",
        "add",
        "core",
        "src/a.rs",
        "--note",
        "Not an additional change.",
    ]);
    repo.ok(&[
        "line",
        "add",
        "core",
        "src/a.rs",
        "10-20",
        "--note",
        "This ordering is deliberate.",
    ]);

    let check = repo.ok(&["map", "check"]);
    assert!(check.contains("Map is complete."), "{check}");

    let show = repo.ok(&["map", "show"]);
    assert!(
        show.contains("block 1  core  \"The change itself\""),
        "{show}"
    );
    assert!(show.contains("Why this exists at all."), "{show}");
    assert!(show.contains("note: Not an additional change."), "{show}");
    assert!(
        show.contains("lines 10-20: This ordering is deliberate."),
        "{show}"
    );
}

#[test]
fn check_fails_while_a_file_belongs_to_nobody() {
    let repo = Repo::new();
    repo.feature();
    repo.write("src/forgotten.rs", "nobody claimed me\n");
    repo.commit("second file");

    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);

    let out = repo.farol(&["map", "check"]);
    assert!(!out.status.success(), "check should fail with a loose file");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("src/forgotten.rs"), "{text}");
    assert!(text.contains("never appears on screen"), "{text}");

    // Marking it skim is a legitimate way to satisfy the rule.
    repo.ok(&["skim", "add", "src/forgotten.rs", "--reason", "mechanical"]);
    assert!(repo.ok(&["map", "check"]).contains("Map is complete."));
}

#[test]
fn deriving_twice_on_the_same_commit_returns_the_same_version() {
    let repo = Repo::new();
    repo.feature();

    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);

    let second = repo.derive();
    assert!(second.contains("already exists"), "{second}");

    // The work from the first run is still there — a run that died halfway just
    // picks up.
    assert!(repo.ok(&["map", "show"]).contains("block 1  core"));
}

#[test]
fn reset_drops_the_newest_version_and_the_previous_one_takes_over() {
    let repo = Repo::new();
    repo.feature();

    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "first",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);

    repo.write("src/b.rs", "second file\n");
    repo.commit("more work");
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "second",
        "--title",
        "t",
        "--context",
        "c",
        "src/b.rs",
    ]);
    assert!(repo.ok(&["map", "show"]).contains("second"));

    let reset = repo.ok(&["map", "reset"]);
    assert!(reset.contains("Deleted"), "{reset}");

    let after = repo.ok(&["map", "show"]);
    assert!(after.contains("first"), "{after}");
    assert!(!after.contains("block 2"), "{after}");
}

#[test]
fn a_new_commit_inherits_the_previous_map() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "Original title",
        "--context",
        "c",
        "src/a.rs",
    ]);

    repo.write("src/a.rs", &numbered(80));
    repo.commit("extend a");

    let derived = repo.derive();
    assert!(derived.contains("Created"), "{derived}");
    assert!(
        derived.contains("Original title"),
        "the new version must inherit the previous blocks:\n{derived}"
    );
}

// ---- line notes across commits -----------------------------------------

#[test]
fn a_note_survives_a_change_far_above_it_by_moving() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);
    repo.ok(&[
        "line",
        "add",
        "core",
        "src/a.rs",
        "40-45",
        "--note",
        "still true",
    ]);

    // Insert five lines at the very top: nothing the note described changed.
    let mut lines: Vec<String> = (1..=5).map(|i| format!("header {i}\n")).collect();
    lines.push(numbered(60));
    repo.write("src/a.rs", &lines.concat());
    repo.commit("prepend a header");

    repo.derive();
    let show = repo.ok(&["map", "show"]);
    assert!(
        show.contains("lines 45-50: still true"),
        "the note should have slid down by five lines:\n{show}"
    );
}

#[test]
fn a_note_whose_code_was_rewritten_is_deactivated_with_its_prose_intact() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);
    repo.ok(&[
        "line",
        "add",
        "core",
        "src/a.rs",
        "40-45",
        "--note",
        "expensive prose",
    ]);

    // Rewrite exactly the lines the note covered.
    let mut lines: Vec<String> = (1..=39).map(|i| format!("line {i}\n")).collect();
    lines.push("completely different\n".into());
    lines.extend((46..=60).map(|i| format!("line {i}\n")));
    repo.write("src/a.rs", &lines.concat());
    repo.commit("rewrite the middle");

    let derived = repo.derive();
    assert!(derived.contains("deactivated"), "{derived}");
    assert!(derived.contains("hunk-overlap"), "{derived}");
    assert!(
        derived.contains("expensive prose"),
        "the text must survive — it is the part that cost context to write:\n{derived}"
    );
    assert!(
        derived.contains("farol line restore core src/a.rs 40-45"),
        "{derived}"
    );

    // It is not shown as a live note anywhere.
    let show = repo.ok(&["map", "show"]);
    assert!(!show.contains("expensive prose"), "{show}");

    // And check refuses to pass while it is undecided.
    let out = repo.farol(&["map", "check"]);
    assert!(!out.status.success(), "check must block on pending orphans");

    repo.ok(&["line", "discard", "core", "src/a.rs", "40-45"]);
    assert!(repo.ok(&["map", "check"]).contains("Map is complete."));
}

#[test]
fn restoring_a_deactivated_note_brings_the_original_text_back() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);
    repo.ok(&[
        "line",
        "add",
        "core",
        "src/a.rs",
        "40-45",
        "--note",
        "worth keeping",
    ]);

    let mut lines: Vec<String> = (1..=39).map(|i| format!("line {i}\n")).collect();
    lines.push("rewritten\n".to_string());
    lines.extend((46..=60).map(|i| format!("line {i}\n")));
    repo.write("src/a.rs", &lines.concat());
    repo.commit("rewrite");

    repo.derive();
    repo.ok(&[
        "line", "restore", "core", "src/a.rs", "40-45", "--range", "38-41",
    ]);

    let show = repo.ok(&["map", "show"]);
    assert!(show.contains("lines 38-41: worth keeping"), "{show}");
}

// ---- validation ---------------------------------------------------------

#[test]
fn a_path_outside_the_review_is_rejected_with_a_suggestion() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&[]);

    let err = repo.fails(&["file", "add", "core", "src/aa.rs"]);
    assert!(err.contains("is not part of this review"), "{err}");
    assert!(
        err.contains("src/a.rs"),
        "a near-miss should be offered back, since hallucinated paths are the \
         most common failure:\n{err}"
    );
}

#[test]
fn a_range_past_the_end_of_the_file_is_rejected() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);

    let err = repo.fails(&["line", "add", "core", "src/a.rs", "500-520", "--note", "x"]);
    assert!(err.contains("outside"), "{err}");
    assert!(err.contains("60 lines"), "{err}");
}

#[test]
fn an_unknown_block_lists_the_ones_that_exist() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&[]);

    let err = repo.fails(&["file", "add", "ghost", "src/a.rs"]);
    assert!(err.contains("unknown block 'ghost'"), "{err}");
    assert!(err.contains("core"), "{err}");
}

#[test]
fn a_duplicate_block_is_rejected_rather_than_silently_merged() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&[]);

    let err = repo.fails(&["block", "add", "core", "--title", "t2", "--context", "c2"]);
    assert!(err.contains("already exists"), "{err}");
}

#[test]
fn malformed_ranges_are_rejected_with_the_expected_shape() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);

    for bad in ["40", "40-10", "abc"] {
        let err = repo.fails(&["line", "add", "core", "src/a.rs", bad, "--note", "x"]);
        assert!(err.contains("invalid range"), "for {bad}: {err}");
    }
}

// ---- ordering -----------------------------------------------------------

#[test]
fn a_new_block_can_be_placed_ahead_of_an_existing_one() {
    let repo = Repo::new();
    repo.feature();
    repo.write("src/b.rs", "second\n");
    repo.commit("b");

    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "alert",
        "--title",
        "The alert",
        "--context",
        "c",
        "src/a.rs",
    ]);
    repo.ok(&[
        "block",
        "add",
        "metric",
        "--title",
        "The metric",
        "--context",
        "c",
        "--before",
        "alert",
        "src/b.rs",
    ]);

    let show = repo.ok(&["map", "show"]);
    let metric = show.find("The metric").unwrap();
    let alert = show.find("The alert").unwrap();
    assert!(
        metric < alert,
        "a block added --before must read first:\n{show}"
    );
}

// ---- serve --------------------------------------------------------------

#[test]
fn serve_refuses_to_start_without_a_map() {
    let repo = Repo::new();
    repo.feature();

    let out = repo.farol(&["serve", "--no-open", "--port", "0"]);
    assert!(!out.status.success(), "serve must not start without a map");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no map for branch"), "{err}");
    assert!(
        err.contains("review-map skill"),
        "the message should say how to fix it:\n{err}"
    );
}

// ---- resilience ---------------------------------------------------------

#[test]
fn a_truncated_map_file_is_discarded_instead_of_crashing() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        "c",
        "src/a.rs",
    ]);

    let maps = repo.path().join(".git/farol/feature-x/maps");
    let file = std::fs::read_dir(&maps)
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path();
    std::fs::write(&file, "{ \"version\": 1, \"blocks\": [").unwrap();

    // The command still runs; the unreadable version is dropped with a warning
    // rather than taking the process down.
    let out = repo.farol(&["map", "show"]);
    assert!(
        out.status.success(),
        "a corrupt file must not be fatal:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unreadable map"), "{err}");
}
