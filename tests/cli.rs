//! End-to-end exercise of the CLI against real repositories.
//!
//! These are the cases a fake `DiffSource` cannot vouch for: how git dirs
//! resolve inside a worktree, what a merge base actually returns once the base
//! branch has moved, and whether the commands refuse the things they promise to
//! refuse.

mod common;

use common::{BIN, Repo, numbered};
use std::path::PathBuf;
use std::process::Command;

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
fn dirty_includes_work_that_is_staged_and_work_that_is_not() {
    let repo = Repo::new();
    repo.feature();

    repo.write("src/staged.rs", "brand new\n");
    repo.git(&["add", "src/staged.rs"]);
    repo.write("src/a.rs", &numbered(70));

    let clean = repo.ok(&["scope"]);
    assert!(!clean.contains("src/staged.rs"), "{clean}");

    let dirty = repo.ok(&["scope", "--dirty"]);
    assert!(
        dirty.contains("src/staged.rs"),
        "staged work counts:\n{dirty}"
    );
    assert!(
        dirty.contains("src/a.rs"),
        "so does an edit that was never staged:\n{dirty}"
    );
}

#[test]
fn dirty_leaves_untracked_files_out() {
    // The point of --dirty is work in progress on files git already knows
    // about; sweeping in scratch files would make the window unpredictable.
    let repo = Repo::new();
    repo.feature();
    repo.write("notes.txt", "scratch\n");

    let dirty = repo.ok(&["scope", "--dirty"]);
    assert!(!dirty.contains("notes.txt"), "{dirty}");
}

#[test]
fn dirty_drops_a_file_that_was_edited_and_then_put_back() {
    // git reports it as touched — the mtime moved — but there is nothing to
    // read, and listing it would send the reviewer to an empty diff.
    let repo = Repo::new();
    repo.feature();
    let original = std::fs::read_to_string(repo.path().join("README.md")).unwrap();
    repo.write("README.md", "changed my mind\n");
    repo.write("README.md", &original);

    let dirty = repo.ok(&["scope", "--dirty"]);
    assert!(!dirty.contains("README.md"), "{dirty}");
}

#[test]
fn dirty_reports_a_tracked_file_deleted_from_the_worktree() {
    let repo = Repo::new();
    repo.feature();
    std::fs::remove_file(repo.path().join("README.md")).unwrap();

    let dirty = repo.ok(&["scope", "--dirty"]);
    assert!(dirty.contains("deleted"), "{dirty}");
    assert!(dirty.contains("README.md"), "{dirty}");
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

#[test]
fn a_file_only_on_the_new_side_is_added() {
    let repo = Repo::new();
    repo.feature();
    let out = repo.ok(&["scope"]);
    assert!(out.contains("added"), "{out}");
    assert!(out.contains("src/a.rs"), "{out}");
}

#[test]
fn a_deleted_file_is_reported_as_deleted() {
    let repo = Repo::new();
    repo.feature();
    repo.git(&["rm", "-q", "README.md"]);
    repo.commit("drop the readme");

    let out = repo.ok(&["scope"]);
    assert!(out.contains("deleted"), "{out}");
    assert!(out.contains("README.md"), "{out}");
}

#[test]
fn a_file_the_branch_left_alone_is_not_in_the_window() {
    let repo = Repo::new();
    repo.write("untouched.rs", "stays\n");
    repo.commit("add a file the branch will not touch");
    repo.feature();

    let out = repo.ok(&["scope"]);
    assert!(
        !out.contains("untouched.rs"),
        "the window is what changed, not what exists:\n{out}"
    );
}

#[test]
fn a_move_that_also_edited_the_file_is_still_one_file_to_read() {
    // Byte-identical moves are the easy case. A move that also touched a line
    // is the common one, and reporting it as an add plus a delete would put the
    // whole file in front of the reviewer twice.
    let repo = Repo::new();
    repo.git(&["checkout", "-q", "-b", "feature/x"]);
    repo.write("src/original.rs", &numbered(40));
    repo.commit("add a file to move later");
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&["merge", "-q", "feature/x"]);
    repo.git(&["checkout", "-q", "feature/x"]);

    repo.git(&["mv", "src/original.rs", "src/moved.rs"]);
    let mut edited = numbered(40);
    edited.push_str("one new line\n");
    repo.write("src/moved.rs", &edited);
    repo.commit("move and edit");

    let out = repo.ok(&["scope"]);
    assert!(
        out.contains("src/moved.rs (was src/original.rs)"),
        "git tracks this rename by similarity, not by byte equality:\n{out}"
    );
    assert!(!out.contains("deleted"), "{out}");
}

#[test]
fn scope_is_listed_in_path_order() {
    let repo = Repo::new();
    repo.git(&["checkout", "-q", "-b", "feature/x"]);
    for name in ["z.rs", "a.rs", "m.rs"] {
        repo.write(name, "x\n");
    }
    repo.commit("three files");

    let out = repo.ok(&["scope"]);
    let order: Vec<usize> = ["a.rs", "m.rs", "z.rs"]
        .iter()
        .map(|p| out.find(p).unwrap_or_else(|| panic!("{p} missing:\n{out}")))
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{out}");
}

// ---- handing a review over ----------------------------------------------

/// The other machine: a clone of the same repository, on the same branch.
///
/// A clone shares every commit, and the base is what an import matches on, so
/// this is the real thing rather than a stand-in. It also needs `main` as a
/// local branch — cloning leaves it as `origin/main` alone, and farol looks the
/// base up by name. Whoever receives a review hits this too.
fn cloned(repo: &Repo) -> std::path::PathBuf {
    let there = repo.path().join("elsewhere");
    repo.git(&[
        "clone",
        "-q",
        repo.path().to_str().unwrap(),
        there.to_str().unwrap(),
    ]);
    repo.git_in(&there, &["branch", "main", "origin/main"]);
    repo.git_in(&there, &["checkout", "-q", "feature/x"]);
    there
}

#[test]
fn a_map_can_be_handed_to_another_machine() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.ok(&["file", "update", "core", "src/a.rs", "--note", "start here"]);

    let head = String::from_utf8_lossy(&repo.git(&["rev-parse", "--short=7", "HEAD"]).stdout)
        .trim()
        .to_string();
    let said = repo.ok(&["map", "export"]);
    let file = repo.path().join(format!("map-{head}.farol.json"));

    assert!(said.contains(&format!("map-{head}.farol.json")), "{said}");
    assert!(file.exists(), "expected the export at {}", file.display());

    // On the other side: nothing of its own, then the map arrives.
    let there = cloned(&repo);
    let empty =
        String::from_utf8_lossy(&repo.farol_in(&there, &["map", "show"]).stdout).into_owned();
    assert!(empty.contains("No map for this branch"), "{empty}");

    let out = repo.farol_in(&there, &["map", "import", file.to_str().unwrap()]);
    let landed = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        landed.contains("Imported the map"),
        "stdout: {landed}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let check = repo.farol_in(&there, &["map", "check"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stdout)
    );
    assert!(
        String::from_utf8_lossy(&repo.farol_in(&there, &["map", "show"]).stdout)
            .contains("start here"),
        "the prose is what was worth sending"
    );
}

#[test]
fn an_imported_map_cannot_overwrite_files_outside_its_store() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    let exported = repo.path().join("review.farol.json");
    repo.ok(&["map", "export", "--out", exported.to_str().unwrap()]);
    let original = std::fs::read_to_string(&exported).unwrap();
    let marker = repo.path().join("import-canary.json");
    repo.write("import-canary.json", "keep this file intact");
    let before = repo.ok(&["map", "show"]);

    for generated_at in [
        "../../../../import-canary".to_string(),
        marker.with_extension("").to_str().unwrap().to_string(),
    ] {
        let mut bundle: serde_json::Value = serde_json::from_str(&original).unwrap();
        bundle["map"]["generated_at"] = generated_at.clone().into();
        std::fs::write(&exported, serde_json::to_vec(&bundle).unwrap()).unwrap();
        let out = repo.farol(&["map", "import", exported.to_str().unwrap()]);

        assert!(
            !out.status.success(),
            "an imported path must be rejected: {generated_at}"
        );
        let error = String::from_utf8_lossy(&out.stderr);
        assert!(
            error.contains("map.generated_at"),
            "the error must identify the invalid field: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            "keep this file intact",
            "an import must never overwrite a file outside its map store"
        );
        assert_eq!(
            repo.ok(&["map", "show"]),
            before,
            "a rejected import must not alter the existing review"
        );
    }

    std::fs::write(&exported, original).unwrap();
    repo.ok(&["map", "import", exported.to_str().unwrap()]);
    assert_eq!(
        repo.ok(&["map", "show"]),
        before,
        "rejecting malicious exports must not prevent a valid import"
    );
}

#[test]
fn a_map_from_another_repository_is_refused() {
    // Nothing but the base commit stands between a map and the wrong tree.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.ok(&["map", "export"]);
    let export = std::fs::read_dir(repo.path())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().ends_with(".farol.json"))
        .expect("the export should be here");

    let stranger = Repo::new();
    stranger.feature();
    let out = stranger.farol(&["map", "import", export.to_str().unwrap()]);

    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("another repository"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
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
    repo.core_note("src/a.rs", "10-20", "This ordering is deliberate.");

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
    repo.core_block(&["src/a.rs"]);

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
    repo.core_block(&["src/a.rs"]);

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
    repo.block("first", &["src/a.rs"]);

    repo.write("src/b.rs", "second file\n");
    repo.commit("more work");
    repo.derive();
    repo.block("second", &["src/b.rs"]);
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
        derived.contains("inherited from"),
        "deriving should say where the version came from:\n{derived}"
    );

    // Read it back with the command whose job that is. Asserting on `derive`'s
    // output was only possible while it printed the whole map, which is what
    // `map show` is for.
    let show = repo.ok(&["map", "show"]);
    assert!(
        show.contains("Original title"),
        "the new version must inherit the previous blocks:\n{show}"
    );
}

// ---- line notes across commits -----------------------------------------

#[test]
fn a_note_survives_a_change_far_above_it_by_moving() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.core_note("src/a.rs", "40-45", "still true");

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
    repo.core_block(&["src/a.rs"]);
    repo.core_note("src/a.rs", "40-45", "expensive prose");

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
fn a_note_can_be_discarded_after_its_file_was_renamed_away() {
    // The commonest way for covered code to be gone is for the file to have
    // moved. Resolving the path against the review first refused exactly those
    // orphans, and `map check` stayed red with no way to clear it.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.core_note("src/a.rs", "40-45", "expensive prose");

    repo.git(&["mv", "src/a.rs", "src/moved.rs"]);
    repo.commit("move it out of the way");

    let derived = repo.derive();
    assert!(derived.contains("deactivated"), "{derived}");
    // The file under its new name still has to be read, so it joins the block;
    // what is left blocking `check` is the orphan alone.
    repo.ok(&["file", "add", "core", "src/moved.rs"]);
    assert!(
        !repo.farol(&["map", "check"]).status.success(),
        "check must block while the orphan is undecided"
    );

    // Named by the path it was written against, which is the only name the map
    // has for it — that path is not in the review any more.
    repo.ok(&["line", "discard", "core", "src/a.rs", "40-45"]);

    assert!(repo.ok(&["map", "check"]).contains("Map is complete."));
}

#[test]
fn restoring_a_deactivated_note_brings_the_original_text_back() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.core_note("src/a.rs", "40-45", "worth keeping");

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
    repo.core_block(&["src/a.rs"]);

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
fn a_malformed_block_name_is_refused_wherever_one_is_named() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);

    // Not just where a block is created — anywhere a block is *referred to*,
    // since those arguments are the same kind of thing.
    for args in [
        vec![
            "block",
            "add",
            "Recover Link",
            "--title",
            "t",
            "--context",
            "c",
        ],
        vec![
            "block",
            "add",
            "other",
            "--title",
            "t",
            "--context",
            "c",
            "--before",
            "Not A Slug",
        ],
        vec!["block", "move", "core", "--after", "Not A Slug"],
        vec![
            "skim",
            "add",
            "src/a.rs",
            "--reason",
            "r",
            "--block",
            "Not A Slug",
        ],
    ] {
        let err = repo.fails(&args);
        assert!(
            err.contains("invalid block name"),
            "for {args:?} the error should name the real problem, not surface as \
             an unknown block later:\n{err}"
        );
    }
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
    repo.core_block(&["src/a.rs"]);

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

// ---- resilience ---------------------------------------------------------

#[test]
fn a_truncated_map_file_is_discarded_instead_of_crashing() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);

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

// ---- taking things back out -------------------------------------------

#[test]
fn a_block_can_be_rewritten_and_reordered_after_the_fact() {
    // Reading order is the point of the map, and the first arrangement is
    // rarely the right one.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.ok(&[
        "block",
        "add",
        "later",
        "--title",
        "Later",
        "--context",
        "c",
    ]);

    repo.ok(&["block", "update", "core", "--title", "The real point"]);
    repo.ok(&["block", "move", "later", "--before", "core"]);

    let out = repo.ok(&["map", "show"]);
    assert!(out.contains("The real point"), "{out}");
    assert!(
        out.find("later").unwrap() < out.find("core").unwrap(),
        "the move should have reordered them:\n{out}"
    );
}

#[test]
fn removing_a_block_keeps_the_prose_that_was_written_inside_it() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.core_note("src/a.rs", "10-12", "worth moving");

    repo.ok(&["block", "remove", "core"]);

    let out = repo.ok(&["map", "derive"]);
    assert!(
        out.contains("worth moving"),
        "a removed block should leave its notes to be replaced, not delete them:\n{out}"
    );
}

#[test]
fn removing_a_file_keeps_its_notes_the_same_way_removing_a_block_does() {
    // Which command you typed should not decide whether your prose survives.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.core_note("src/a.rs", "10-12", "still true");

    repo.ok(&["file", "remove", "core", "src/a.rs"]);

    let out = repo.ok(&["map", "derive"]);
    assert!(out.contains("still true"), "{out}");
}

#[test]
fn notes_can_be_rewritten_and_withdrawn() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.ok(&[
        "file",
        "update",
        "core",
        "src/a.rs",
        "--note",
        "first thought",
    ]);
    repo.core_note("src/a.rs", "10-12", "first thought");

    repo.ok(&[
        "file",
        "update",
        "core",
        "src/a.rs",
        "--note",
        "what I meant",
    ]);
    repo.ok(&[
        "line",
        "update",
        "core",
        "src/a.rs",
        "10-12",
        "--note",
        "what I meant",
    ]);

    let out = repo.ok(&["map", "show"]);
    assert!(!out.contains("first thought"), "{out}");
    assert_eq!(out.matches("what I meant").count(), 2, "{out}");

    repo.ok(&["line", "remove", "core", "src/a.rs", "10-12"]);
    let out = repo.ok(&["map", "show"]);
    assert!(
        !out.contains("lines 10-12"),
        "the line note should be gone, while the file note stays:\n{out}"
    );
    assert!(out.contains("what I meant"), "{out}");
}

#[test]
fn a_skim_entry_can_be_taken_back_out() {
    let repo = Repo::new();
    repo.feature();
    repo.write("Cargo.lock", "checksums\n");
    repo.commit("regenerate");
    repo.derive();
    repo.ok(&["skim", "add", "Cargo.lock", "--reason", "regenerated"]);
    assert!(repo.ok(&["map", "show"]).contains("Cargo.lock"));

    repo.ok(&["skim", "remove", "Cargo.lock"]);

    assert!(!repo.ok(&["map", "show"]).contains("Cargo.lock"));
}

#[test]
fn removing_something_that_is_not_there_fails_instead_of_pretending() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);

    assert!(repo.fails(&["block", "remove", "nope"]).contains("core"));
    repo.fails(&["file", "remove", "core", "README.md"]);
    repo.fails(&["line", "remove", "core", "src/a.rs", "10-12"]);
}

// ---- history ------------------------------------------------------------

#[test]
fn a_map_reports_how_far_head_has_moved_past_it() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();

    repo.write("src/a.rs", &numbered(61));
    repo.commit("one more");
    repo.write("src/a.rs", &numbered(62));
    repo.commit("and another");

    let out = repo.ok(&["map", "show"]);
    assert!(
        out.contains("2 commits behind"),
        "map show should say how stale it is:\n{out}"
    );
}

#[test]
fn a_merge_does_not_inflate_the_distance() {
    // Distance is `target..HEAD`, not "how many commits until I stumble on it":
    // work merged in from a side branch is not distance travelled since the map.
    let repo = Repo::new();
    repo.feature();
    repo.derive();

    repo.git(&["checkout", "-q", "-b", "side"]);
    for i in 1..=5 {
        repo.write("src/side.rs", &numbered(i));
        repo.commit("side work");
    }
    repo.git(&["checkout", "-q", "feature/x"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge side", "side"]);

    let out = repo.ok(&["map", "show"]);
    assert!(
        out.contains("6 commits behind"),
        "five side commits plus the merge, counted once each:\n{out}"
    );
}

#[test]
fn the_map_of_a_commit_that_is_no_longer_reachable_is_not_adopted() {
    // After a reset that throws the commit away, its map must not be picked up
    // as the current one — it describes code that is no longer on this branch.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);

    repo.write("src/a.rs", &numbered(70));
    repo.commit("rewrite");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    repo.write("src/a.rs", &numbered(80));
    repo.commit("different direction");

    let out = repo.ok(&["map", "show"]);
    assert!(
        out.contains("core"),
        "the map from the still-reachable commit is the one that applies:\n{out}"
    );
}

#[test]
fn a_binary_file_is_not_decoded_into_lines_that_do_not_exist() {
    // from_utf8_lossy on a PNG produces mojibake, and the screen would render
    // it as if it were code. git refuses to diff binary content; so do we.
    let repo = Repo::new();
    repo.git(&["checkout", "-q", "-b", "feature/x"]);
    std::fs::write(
        repo.path().join("logo.png"),
        (0u8..=255).cycle().take(4000).collect::<Vec<u8>>(),
    )
    .unwrap();
    repo.commit("add a binary");

    let out = repo.ok(&["scope"]);
    assert!(out.contains("logo.png"), "{out}");
    assert!(
        out.contains("+0") && out.contains("-0"),
        "a binary file has no lines to count:\n{out}"
    );
}

#[test]
fn a_file_marked_not_diffable_is_left_alone() {
    // `-diff` in .gitattributes is how a repository says "do not read this line
    // by line" — generated code, vendored bundles, lockfiles.
    let repo = Repo::new();
    repo.git(&["checkout", "-q", "-b", "feature/x"]);
    repo.write(".gitattributes", "generated.txt -diff\n");
    repo.write("generated.txt", &numbered(50));
    repo.commit("add generated output");

    let out = repo.ok(&["scope"]);
    assert!(out.contains("generated.txt"), "{out}");
    assert!(
        !out.contains("+50"),
        "the repository asked for this not to be diffed:\n{out}"
    );
}

#[test]
fn resetting_the_only_version_says_the_branch_is_unmapped_again() {
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);

    let out = repo.ok(&["map", "reset"]);

    assert!(out.contains("Deleted the map version"), "{out}");
    assert!(out.contains("unmapped"), "{out}");
    assert!(repo.fails(&["map", "check"]).contains("feature/x"));
}

#[test]
fn resetting_when_there_is_nothing_here_to_delete_says_so() {
    // `map reset` on a commit that was never mapped is a no-op, and silence
    // would look like it had deleted something.
    let repo = Repo::new();
    repo.feature();

    let out = repo.ok(&["map", "reset"]);

    assert!(out.contains("no map version for this commit"), "{out}");
}

// ---- output --------------------------------------------------------------

#[test]
fn quitting_a_pager_early_does_not_end_in_a_backtrace() {
    // Rust starts life with SIGPIPE ignored, which turns the reader going away
    // into a write error and then a panic. `farol map show | head` is ordinary
    // use, and it should end, not crash. The context is oversized on purpose:
    // below the pipe buffer the write succeeds and there is nothing to catch.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    let long = "explaining at length. ".repeat(5_000);
    repo.ok(&[
        "block",
        "add",
        "core",
        "--title",
        "t",
        "--context",
        &long,
        "src/a.rs",
    ]);

    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("{BIN} map show | head -1"))
        .current_dir(repo.path())
        .output()
        .expect("the pipeline should run");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "a closed pipe should not be a crash:\n{stderr}"
    );
}

#[test]
fn showing_the_map_admits_there_is_a_decision_waiting() {
    // The session reads the state with `map show`. If a deactivated note only
    // appeared in `map derive`, it would finish thinking it was done.
    let repo = Repo::new();
    repo.feature();
    repo.derive();
    repo.core_block(&["src/a.rs"]);
    repo.core_note("src/a.rs", "10-12", "worth keeping");

    // Rewrite exactly under the note.
    let rewritten = numbered(40).replace("line 11\n", "REWRITTEN\n");
    repo.write("src/a.rs", &rewritten);
    repo.commit("rewrite under the note");
    repo.derive();

    let show = repo.ok(&["map", "show"]);

    assert!(show.contains("1 line note is deactivated"), "{show}");
    assert!(show.contains("farol map derive"), "{show}");
}

// ---- sending the review ---------------------------------------------------

#[test]
fn github_status_says_a_repository_with_no_remote_has_nowhere_to_send() {
    // The one state a test can reach without a network: no remote at all.
    // Reported as a state of its own rather than as an error, because it is
    // not a step on the way to being ready.
    let repo = Repo::new();
    repo.feature();

    let out = repo.farol(&["github", "status"]);

    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with("no remote:"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn github_status_exits_non_zero_while_the_review_cannot_go() {
    // The whole point of the command for a session: branch on the code, read
    // the words only to say what happened.
    let repo = Repo::new();
    repo.feature();

    let out = repo.farol(&["github", "status"]);

    assert!(
        !out.status.success(),
        "a review that cannot go exits non-zero"
    );
}

#[test]
fn github_review_needs_a_verdict_and_takes_only_one() {
    // The verdict is the decision the review is, so there is no default, and
    // two of them is a contradiction rather than a preference.
    let repo = Repo::new();
    repo.feature();

    let none = repo.fails(&["github", "review"]);
    let both = repo.fails(&["github", "review", "--approve", "--comment"]);

    assert!(none.contains("--comment"), "{none}");
    assert!(both.contains("cannot be used with"), "{both}");
}

#[test]
fn github_review_refuses_where_there_is_nowhere_to_send() {
    let repo = Repo::new();
    repo.feature();

    let err = repo.fails(&["github", "review", "--approve"]);

    assert!(err.contains("no remote"), "{err}");
}

// ---- comments on either side of the diff --------------------------------

/// A branch that deletes one file whole and edits another in place, which is
/// what gives the review an old side worth asking about.
fn removing() -> Repo {
    let repo = Repo::new();
    repo.write("src/gone.rs", &numbered(6));
    repo.write("src/keep.rs", &numbered(20));
    repo.commit("what the branch will take away and change");
    repo.feature();
    repo.git(&["rm", "-q", "src/gone.rs"]);
    repo.write(
        "src/keep.rs",
        &numbered(20).replace("line 5\n", "line five\n"),
    );
    repo.commit("delete one file and edit another");
    repo
}

#[test]
fn a_comment_can_be_written_on_the_old_side_of_a_file_that_is_gone() {
    // A deleted file has no new side at all, so refusing the old one would
    // leave the one kind of change nobody can ask a question about. The list
    // says which side, because the numbers alone would name two places.
    let repo = removing();

    let wrote = repo.ok(&[
        "comment",
        "add",
        "src/gone.rs",
        "2-4",
        "--side",
        "old",
        "--text",
        "Where did this go?",
    ]);
    let list = repo.ok(&["comment", "list"]);

    assert!(wrote.contains("src/gone.rs:2-4 (old)"), "{wrote}");
    assert!(list.contains("src/gone.rs:2-4 (old)"), "{list}");
    assert!(list.contains("    Where did this go?"), "{list}");
}

#[test]
fn a_comment_defaults_to_the_side_the_reviewer_is_reading() {
    // No flag is the new side, which is what every comment written before
    // there was a choice meant — and what the unmarked form goes on meaning.
    let repo = removing();

    repo.ok(&[
        "comment",
        "add",
        "src/keep.rs",
        "5-5",
        "--text",
        "Why this?",
    ]);
    let list = repo.ok(&["comment", "list"]);

    assert!(list.contains("src/keep.rs:5\n"), "{list}");
    assert!(!list.contains("(old)"), "{list}");
}

#[test]
fn the_two_sides_of_one_line_are_two_separate_comments() {
    // Line 5 was rewritten, so it is in the diff on both sides. Neither the
    // store nor the list may fold them into one.
    let repo = removing();

    repo.ok(&[
        "comment",
        "add",
        "src/keep.rs",
        "5-5",
        "--side",
        "old",
        "--text",
        "Why was this here?",
    ]);
    repo.ok(&[
        "comment",
        "add",
        "src/keep.rs",
        "5-5",
        "--side",
        "new",
        "--text",
        "Why is this here?",
    ]);
    let list = repo.ok(&["comment", "list"]);

    assert!(list.contains("src/keep.rs:5 (old)"), "{list}");
    assert!(list.contains("Why was this here?"), "{list}");
    assert!(list.contains("Why is this here?"), "{list}");
}

#[test]
fn a_comment_off_the_diff_is_refused_in_the_terms_of_the_side_it_was_asked_on() {
    // The same numbers are in the diff on one side and not on the other, so a
    // refusal that did not say which side would read as farol being wrong.
    let repo = removing();

    let err = repo.fails(&[
        "comment",
        "add",
        "src/keep.rs",
        "18-18",
        "--side",
        "old",
        "--text",
        "Why?",
    ]);

    assert!(err.contains("old side"), "{err}");
    assert!(err.contains("src/keep.rs"), "{err}");
    assert!(repo.ok(&["comment", "list"]).contains("No comments yet"));
}

#[test]
fn a_file_that_is_gone_has_no_new_side_left_to_ask_about() {
    // Nothing of it survives the change, so the new side is a file of no
    // length at all and every line of it is past the end.
    let repo = removing();

    let err = repo.fails(&[
        "comment",
        "add",
        "src/gone.rs",
        "2-4",
        "--side",
        "new",
        "--text",
        "Why?",
    ]);

    assert!(err.contains("0 lines"), "{err}");
    assert!(repo.ok(&["comment", "list"]).contains("No comments yet"));
}

#[test]
fn a_side_that_is_neither_is_refused_before_anything_is_written() {
    let repo = removing();

    let err = repo.fails(&[
        "comment",
        "add",
        "src/gone.rs",
        "2-4",
        "--side",
        "left",
        "--text",
        "Why?",
    ]);

    assert!(err.contains("'old' or 'new'"), "{err}");
}

#[test]
fn github_publication_explains_when_gh_is_not_installed() {
    let repo = Repo::new();
    repo.feature();
    repo.git(&[
        "remote",
        "add",
        "origin",
        "https://github.example.test/owner/repo.git",
    ]);
    let empty = tempfile::tempdir().unwrap();
    let output = repo
        .command(&["github", "status"])
        .env("PATH", empty.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Install GitHub CLI"));
    assert!(
        !repo
            .path()
            .join(".farol-config/farol/github-credential.json")
            .exists()
    );
}

#[cfg(unix)]
#[test]
fn github_status_uses_the_remote_host_and_guides_gh_login_without_leaking_output() {
    use std::os::unix::fs::PermissionsExt;
    let repo = Repo::new();
    repo.feature();
    repo.git(&[
        "remote",
        "add",
        "origin",
        "https://github.example.test/owner/repo.git",
    ]);
    let bin = tempfile::tempdir().unwrap();
    let gh = bin.path().join("gh");
    std::fs::write(
        &gh,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > gh-args.txt\necho fixture-secret >&2\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = repo
        .command(&["github", "status"])
        .env("PATH", bin.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gh auth login --hostname github.example.test"));
    assert!(!stdout.contains("fixture-secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-secret"));
    assert_eq!(
        std::fs::read_to_string(repo.path().join("gh-args.txt")).unwrap(),
        "auth\ntoken\n--hostname\ngithub.example.test\n"
    );
}

#[test]
fn marks_only_is_an_explicit_cli_action() {
    let repo = Repo::new();
    repo.feature();
    let output = repo.farol(&["github", "ticks"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no remote"));
}
