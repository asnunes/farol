//! The repository these tests run against.
//!
//! Shared because two integration crates need the same setup, and two copies
//! of it would drift into disagreeing about what a review looks like.

#![allow(dead_code)] // each integration crate uses a different slice of it

use std::path::Path;
use std::process::{Command, Output};

pub const BIN: &str = env!("CARGO_BIN_EXE_farol");

pub struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    /// A repository with `main` holding one commit.
    pub fn new() -> Self {
        Self::with_default_branch("main")
    }

    pub fn with_default_branch(branch: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repo { dir };
        repo.git(&["init", "-q", "--initial-branch", branch]);
        repo.git(&["config", "user.email", "test@farol"]);
        repo.git(&["config", "user.name", "farol test"]);
        // Seeded with the directory it lives in, so two repositories built in
        // the same second are not the same repository. Everything else about
        // them is identical — same file, same author, same message — and git
        // hashes the timestamp only to the second, so without this they share
        // a base commit and a test about telling them apart cannot.
        repo.write("README.md", &format!("start\n{}\n", repo.path().display()));
        repo.git(&["add", "."]);
        repo.commit("initial");
        repo
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A registry of running servers belonging to this test alone.
    ///
    /// Without it a test run would list — and stop — the servers of whoever is
    /// running it, and two tests would fight over the same ports.
    pub fn state_dir(&self) -> std::path::PathBuf {
        self.path().join(".farol-state")
    }

    pub fn git(&self, args: &[&str]) -> Output {
        self.git_in(self.path(), args)
    }

    pub fn git_in(&self, cwd: &Path, args: &[&str]) -> Output {
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

    pub fn write(&self, rel: &str, contents: &str) {
        self.write_in(self.path(), rel, contents)
    }

    pub fn write_in(&self, root: &Path, rel: &str, contents: &str) {
        let full = root.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, contents).unwrap();
    }

    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message, "--no-gpg-sign"]);
    }

    pub fn farol(&self, args: &[&str]) -> Output {
        self.farol_in(self.path(), args)
    }

    pub fn farol_in(&self, cwd: &Path, args: &[&str]) -> Output {
        self.command(args)
            .current_dir(cwd)
            .output()
            .expect("farol should run")
    }

    /// Run here, but against another repository's registry — which is what a
    /// single machine looks like to two repositories.
    pub fn farol_sharing(&self, registry: &Path, args: &[&str]) -> Output {
        self.command(args)
            .env("FAROL_STATE_DIR", registry)
            .output()
            .expect("farol should run")
    }

    /// `farol`, pointed at this repository's own state.
    pub fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(BIN);
        cmd.args(args)
            .current_dir(self.path())
            .env("FAROL_STATE_DIR", self.state_dir());
        cmd
    }

    /// Run and require success, returning stdout.
    pub fn ok(&self, args: &[&str]) -> String {
        let out = self.farol(args);
        assert!(
            out.status.success(),
            "farol {args:?} should have succeeded:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Run and require failure, returning stderr.
    pub fn fails(&self, args: &[&str]) -> String {
        let out = self.farol(args);
        assert!(
            !out.status.success(),
            "farol {args:?} should have failed but printed:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
        String::from_utf8_lossy(&out.stderr).into_owned()
    }

    /// Start the map for the current commit.
    pub fn derive(&self) -> String {
        self.ok(&["map", "derive"])
    }

    /// The block most tests need: one block holding the given files.
    pub fn core_block(&self, paths: &[&str]) {
        let mut args = vec!["block", "add", "core", "--title", "t", "--context", "c"];
        args.extend_from_slice(paths);
        self.ok(&args);
    }

    /// A branch off main with one file changed, ready to review.
    pub fn feature(&self) -> &Self {
        self.git(&["checkout", "-q", "-b", "feature/x"]);
        self.write("src/a.rs", &numbered(60));
        self.commit("add a");
        self
    }
}

/// A file with predictable line numbers, so ranges in tests mean something.
pub fn numbered(lines: usize) -> String {
    (1..=lines)
        .map(|i| format!("line {i}\n"))
        .collect::<String>()
}
