//! A real repository to test against.
//!
//! Git is the thing under test in this layer, so a fake would only assert what
//! we already believe. Kept in one place for the same reason the fakes are: two
//! fixtures drift, and then two tests disagree about what git does.

use super::git::Git;
use super::gix_source::{GixSource, ScopeRequest};

/// A real repository, because every one of these answers comes from git and
/// a fake would only be asserting what we already believe.
pub(super) struct Fixture {
    pub dir: tempfile::TempDir,
}

impl Fixture {
    /// `main` with one commit holding `README.md`.
    pub fn new() -> Self {
        let f = Fixture {
            dir: tempfile::tempdir().unwrap(),
        };
        f.git(&["init", "-q", "--initial-branch", "main"]);
        f.git(&["config", "user.email", "test@farol"]);
        f.git(&["config", "user.name", "farol test"]);
        f.write("README.md", "start\n");
        f.commit("initial");
        f
    }

    pub fn git(&self, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(self.dir.path())
            .output()
            .expect("git should run");
        assert!(
            out.status.success(),
            "git {args:?} failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    pub fn write(&self, rel: &str, contents: &str) {
        let full = self.dir.path().join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, contents).unwrap();
    }

    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message, "--no-gpg-sign"]);
    }

    pub fn open(&self) -> Git {
        Git::new(gix::open(self.dir.path()).expect("the repository should open"))
    }

    pub fn sha(&self, rev: &str) -> gix::ObjectId {
        self.open().resolve(rev).unwrap()
    }
}

impl Fixture {
    /// A branch off `main`, so there is something to review.
    pub fn on_branch(&self, name: &str) -> &Self {
        self.git(&["checkout", "-q", "-b", name]);
        self
    }

    /// The source as the composition root would build it.
    pub fn source(&self, branch: &str, req: ScopeRequest) -> GixSource {
        GixSource::open(
            gix::open(self.dir.path()).expect("the repository should open"),
            branch,
            &req,
        )
        .expect("the scope should resolve")
    }
}
