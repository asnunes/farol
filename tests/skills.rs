//! The distributed checkout helper must preserve the reviewer's local work.

mod common;

use common::Repo;
use std::process::{Command, Output};

#[test]
fn the_context_helper_checks_out_the_pr_then_recognizes_it() {
    let repo = Repo::new();
    repo.feature();
    let head = repo.head();
    repo.git(&["checkout", "-q", "main"]);

    let first = repo.pr_context(&head);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(String::from_utf8_lossy(&first.stdout).contains("status=checked_out"));
    assert_eq!(repo.head(), head);

    let second = repo.pr_context(&head);
    assert!(second.status.success());
    let output = String::from_utf8_lossy(&second.stdout);
    assert!(output.contains("status=aligned"), "{output}");
    assert!(output.contains("worktree_dirty=false"), "{output}");
}

#[test]
fn a_required_checkout_leaves_dirty_files_and_the_current_branch_alone() {
    let repo = Repo::new();
    repo.feature();
    let pr_head = repo.head();
    repo.git(&["checkout", "-q", "main"]);
    let before = repo.head();
    repo.write("README.md", "uncommitted work\n");
    repo.write("local-notes.txt", "untracked work\n");

    let output = repo.pr_context(&pr_head);

    assert_eq!(output.status.code(), Some(65));
    assert_eq!(repo.head(), before, "a refused checkout must not move HEAD");
    assert_eq!(
        std::fs::read_to_string(repo.path().join("README.md")).unwrap(),
        "uncommitted work\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("local-notes.txt")).unwrap(),
        "untracked work\n"
    );
}

#[test]
fn an_aligned_dirty_checkout_is_reported_without_discarding_work() {
    let repo = Repo::new();
    repo.feature();
    let head = repo.head();
    repo.write("src/a.rs", "local edit\n");

    let output = repo.pr_context(&head);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("worktree_dirty=true"));
    assert_eq!(
        std::fs::read_to_string(repo.path().join("src/a.rs")).unwrap(),
        "local edit\n"
    );
    assert_eq!(repo.head(), head);
}

#[test]
fn a_local_commit_ahead_of_the_pr_is_not_reset() {
    let repo = Repo::new();
    repo.feature();
    let pr_head = repo.head();
    repo.git(&[
        "commit",
        "--allow-empty",
        "--no-gpg-sign",
        "-qm",
        "local-only commit",
    ]);
    let local_head = repo.head();

    let output = repo.pr_context(&pr_head);

    assert_eq!(output.status.code(), Some(70));
    assert_eq!(
        repo.head(),
        local_head,
        "a mismatch must preserve the local commit"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Refusing to force-reset"));
}

impl Repo {
    fn head(&self) -> String {
        String::from_utf8(self.git(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string()
    }

    fn pr_context(&self, pr_head: &str) -> Output {
        use std::os::unix::fs::PermissionsExt;

        // Git is real; only GitHub metadata and checkout dispatch are replaced.
        // Keep the stub outside the repository so it does not make it dirty.
        let commands = tempfile::tempdir().unwrap();
        let gh = commands.path().join("gh");
        std::fs::write(
            &gh,
            r#"#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
    "pr view") printf 'feature/x\t%s\n' "$FAROL_TEST_PR_HEAD" ;;
    "pr checkout") git checkout -q feature/x ;;
    *) echo "unexpected gh invocation" >&2; exit 1 ;;
esac
"#,
        )
        .unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
        let paths = std::iter::once(commands.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()))
            .collect::<Vec<_>>();
        Command::new("bash")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/skills/farol-pr-context/scripts/ensure_pr_checkout.sh"
            ))
            .arg("123")
            .env("PATH", std::env::join_paths(paths).unwrap())
            .env("FAROL_TEST_PR_HEAD", pr_head)
            .current_dir(self.path())
            .output()
            .unwrap()
    }
}
