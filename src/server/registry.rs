//! Which farol servers are running, and where.
//!
//! A file per server rather than one shared list: two `serve` starting at the
//! same moment would write over each other, and there is nothing here worth
//! locking for. The file is named by the port, which is the one thing that
//! cannot collide while both are alive.
//!
//! The registry is a cache of what the operating system already knows, so it is
//! never trusted on its own: every read checks that the process is still there
//! and drops the entries that are not. A server killed with `-9`, or gone with
//! its terminal, leaves a file behind and nothing else.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::health;
use crate::error::{Error, Result};
use crate::shared::paths::write_json;

/// Where the registry lives, when the environment does not say otherwise.
/// Overridable so a test never writes into the registry of the machine running
/// it — and never stops a server somebody is reading.
const STATE_DIR: &str = "FAROL_STATE_DIR";

/// How long `stop` waits for a server to leave on its own before reporting that
/// it did not. Shutdown is a socket closing, so this is generous.
const TO_STOP: Duration = Duration::from_secs(3);

const POLL: Duration = Duration::from_millis(20);

/// One running server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub port: u16,
    pub pid: u32,
    /// The working tree, not the git dir: it is what the reader recognises in a
    /// list of open reviews.
    pub repo: PathBuf,
    pub branch: String,
    pub base: String,
}

impl ServerEntry {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

pub struct Registry {
    root: PathBuf,
}

impl Registry {
    pub fn open() -> Result<Self> {
        match std::env::var_os(STATE_DIR) {
            Some(dir) => Self::at(dir),
            None => Self::at(default_root()?),
        }
    }

    pub fn at(root: impl Into<PathBuf>) -> Result<Self> {
        let registry = Self { root: root.into() };
        std::fs::create_dir_all(registry.servers_dir())?;
        std::fs::create_dir_all(registry.logs_dir())?;
        Ok(registry)
    }

    pub fn servers_dir(&self) -> PathBuf {
        self.root.join("servers")
    }

    /// Where a detached server's output goes. Nobody is watching its terminal,
    /// so a crash has to leave something behind to read.
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn log_file(&self, port: u16) -> PathBuf {
        self.logs_dir().join(format!("{port}.log"))
    }

    fn file(&self, port: u16) -> PathBuf {
        self.servers_dir().join(format!("{port}.json"))
    }

    pub fn register(&self, entry: &ServerEntry) -> Result<()> {
        write_json(&self.file(entry.port), entry)
    }

    pub fn deregister(&self, port: u16) -> Result<()> {
        forget(&self.file(port))
    }

    /// Every server that is actually running, in port order.
    ///
    /// Entries whose process is gone are deleted on the way past, so the
    /// registry cleans itself up by being read rather than needing a sweep.
    pub fn running(&self) -> Result<Vec<ServerEntry>> {
        let mut entries = Vec::new();
        for file in std::fs::read_dir(self.servers_dir())?.flatten() {
            let path = file.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            match read(&path) {
                Some(entry) if alive(entry.pid) => entries.push(entry),
                _ => forget(&path)?,
            }
        }
        entries.sort_by_key(|e| e.port);
        Ok(entries)
    }

    /// The servers that are actually serving.
    ///
    /// `running` believes the file as long as the process is there; this asks
    /// the port. They disagree whenever a server stopped serving without
    /// ending — which it does when it is asked to stop while a browser holds
    /// the change stream open.
    ///
    /// An entry that fails here is left on disk on purpose. It is not reported
    /// as a running review, but `stop` still has to be able to reach it, and
    /// deleting the file would leave the process with nothing pointing at it.
    pub fn answering(&self) -> Result<Vec<ServerEntry>> {
        Ok(self
            .running()?
            .into_iter()
            .filter(health::answers)
            .collect())
    }

    /// A server already showing this branch of this repository, if there is one.
    ///
    /// Answering, not merely registered: handing back a port that stopped
    /// serving would send the reviewer to an empty tab.
    pub fn serving(&self, repo: &Path, branch: &str) -> Result<Option<ServerEntry>> {
        Ok(self
            .answering()?
            .into_iter()
            .find(|e| e.repo == repo && e.branch == branch))
    }

    /// Ask a server to stop, and wait until it has.
    ///
    /// `SIGTERM`, never `SIGKILL`: the server shuts down gracefully, which is
    /// what lets it take its own entry out of the registry.
    pub fn stop(&self, entry: &ServerEntry) -> Result<()> {
        // SAFETY: `kill` with a signal is a plain syscall; the worst a stale pid
        // can do is address a process that no longer exists, which is `ESRCH`.
        unsafe { libc::kill(entry.pid as libc::pid_t, libc::SIGTERM) };

        let deadline = Instant::now() + TO_STOP;
        while alive(entry.pid) {
            if Instant::now() >= deadline {
                return Err(Error::msg(format!(
                    "the server on port {} did not stop — its process is {}",
                    entry.port, entry.pid
                )));
            }
            std::thread::sleep(POLL);
        }

        self.deregister(entry.port)?;
        forget(&self.log_file(entry.port))?;
        Ok(())
    }
}

/// `$XDG_STATE_HOME/farol`, or the path that variable defaults to.
fn default_root() -> Result<PathBuf> {
    if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(state).join("farol"));
    }
    let home = std::env::var_os("HOME")
        .ok_or_else(|| Error::msg("no HOME to keep the list of running servers under"))?;
    Ok(PathBuf::from(home).join(".local/state/farol"))
}

/// Whether the process behind an entry is still there.
///
/// Signal 0 checks for the process without touching it. `EPERM` means it exists
/// and belongs to somebody else, which still counts as running. A recycled pid
/// would read as alive — the window for that is small enough to live with, and
/// the alternative is recording a start time and trusting clocks.
fn alive(pid: u32) -> bool {
    // SAFETY: signal 0 sends nothing; it only asks whether the pid could be
    // signalled.
    let sent = unsafe { libc::kill(pid as libc::pid_t, 0) };
    sent == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn read(path: &Path) -> Option<ServerEntry> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Delete, treating "it was already gone" as success — which is the state the
/// caller wanted either way.
fn forget(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A registry of its own, so tests never see the machine's real one. That
    /// `open()` reads the same directory out of the environment is proved from
    /// outside, in `tests/server.rs`, where the whole point is a separate
    /// process.
    fn registry() -> (tempfile::TempDir, Registry) {
        let dir = tempfile::tempdir().unwrap();
        let registry = Registry::at(dir.path()).unwrap();
        (dir, registry)
    }

    fn entry(port: u16) -> ServerEntry {
        ServerEntry {
            port,
            pid: std::process::id(),
            repo: PathBuf::from("/repo"),
            branch: "feature/x".into(),
            base: "main".into(),
        }
    }

    /// The pid of a process that has certainly finished.
    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        pid
    }

    #[test]
    fn a_registered_server_comes_back_whole() {
        let (_dir, registry) = registry();
        registry.register(&entry(4600)).unwrap();

        assert_eq!(registry.running().unwrap(), vec![entry(4600)]);
    }

    #[test]
    fn servers_are_listed_in_port_order_however_they_were_written() {
        // The list is read from a directory, and directory order is nobody's
        // idea of an order.
        let (_dir, registry) = registry();
        for port in [4603, 4600, 4601] {
            registry.register(&entry(port)).unwrap();
        }

        let ports: Vec<_> = registry.running().unwrap().iter().map(|e| e.port).collect();
        assert_eq!(ports, vec![4600, 4601, 4603]);
    }

    #[test]
    fn a_server_whose_process_is_gone_is_dropped_and_its_file_deleted() {
        // A server killed with -9, or gone with its terminal, never got to take
        // itself out. Reading the registry is what cleans it up.
        let (_dir, registry) = registry();
        let mut stale = entry(4600);
        stale.pid = dead_pid();
        registry.register(&stale).unwrap();

        assert!(registry.running().unwrap().is_empty());
        assert!(
            !registry.file(4600).exists(),
            "the stale entry should have been deleted, not merely skipped"
        );
    }

    #[test]
    fn rubbish_in_the_registry_reads_as_no_server_rather_than_an_error() {
        // Half a file from an older version is not worth failing a listing for.
        let (_dir, registry) = registry();
        std::fs::write(registry.file(4600), "{ not json").unwrap();

        assert!(registry.running().unwrap().is_empty());
    }

    #[test]
    fn a_repository_and_branch_find_the_server_already_showing_them() {
        // Answering, so something has to be there to answer: `serving` is what
        // hands a second `farol serve` back the review already open, and a port
        // that stopped serving must not be handed to anybody.
        let (_dir, registry) = registry();
        let port = super::health::tests::server_answering(Some(entry(4600)));
        let mut open = entry(port);
        open.pid = std::process::id();
        registry.register(&open).unwrap();

        assert!(
            registry
                .serving(Path::new("/repo"), "feature/x")
                .unwrap()
                .is_some()
        );
        assert!(
            registry
                .serving(Path::new("/repo"), "other")
                .unwrap()
                .is_none(),
            "another branch of the same repository is another review"
        );
        assert!(
            registry
                .serving(Path::new("/elsewhere"), "feature/x")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn stopping_a_server_that_is_already_gone_still_clears_it_away() {
        let (_dir, registry) = registry();
        let mut stale = entry(4600);
        stale.pid = dead_pid();
        registry.register(&stale).unwrap();
        std::fs::write(registry.log_file(4600), "old output").unwrap();

        registry.stop(&stale).unwrap();

        assert!(!registry.file(4600).exists());
        assert!(!registry.log_file(4600).exists(), "the log goes with it");
    }

    #[test]
    fn opening_a_registry_creates_the_places_it_writes_to() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Registry::at(dir.path().join("state")).unwrap();

        assert!(registry.servers_dir().is_dir());
        assert!(registry.logs_dir().is_dir());
    }
}
