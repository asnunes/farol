use std::path::PathBuf;
use std::process::Command;

use crate::agent::domain::{AgentChannel, AgentError};

const HARNESS: &str = "Codex";

/// Codex's own queue, reached through its CLI.
///
/// `codex queue` finds the daemon itself and, without one, writes to the queue
/// the session reads, so farol neither speaks the app-server protocol nor
/// manages a daemon. The message waits for the current turn; it never cuts one.
pub struct CodexQueue {
    executable: PathBuf,
    thread: String,
}

impl CodexQueue {
    pub fn new(executable: impl Into<PathBuf>, thread: String) -> Self {
        Self {
            executable: executable.into(),
            thread,
        }
    }
}

impl AgentChannel for CodexQueue {
    fn harness(&self) -> &'static str {
        HARNESS
    }

    fn send(&self, text: &str) -> Result<(), AgentError> {
        // `--message=` joined, so a text that opens with `-` is not read as a flag.
        let output = Command::new(&self.executable)
            .args(["queue", "--thread", &self.thread])
            .arg(format!("--message={text}"))
            .output()
            .map_err(|_| AgentError::Unavailable)?;
        if output.status.success() {
            return Ok(());
        }
        let said = String::from_utf8_lossy(&output.stderr);
        let said = match said.trim().is_empty() {
            true => String::from_utf8_lossy(&output.stdout).into_owned(),
            false => said.into_owned(),
        };
        Err(AgentError::Refused {
            harness: HARNESS,
            why: said
                .lines()
                .next()
                .unwrap_or("no reason given")
                .trim()
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn codex(dir: &std::path::Path, script: &str) -> PathBuf {
        let exe = dir.join("codex");
        std::fs::write(&exe, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        exe
    }

    #[test]
    fn the_thread_and_message_reach_codex_without_a_shell() {
        let dir = tempfile::tempdir().unwrap();
        let record = dir.path().join("args");
        let exe = codex(
            dir.path(),
            &format!("printf '%s\\n' \"$@\" > {}", record.display()),
        );

        CodexQueue::new(exe, "thread-1".into())
            .send("-starts with a dash; $(not run)")
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(record).unwrap(),
            "queue\n--thread\nthread-1\n--message=-starts with a dash; $(not run)\n"
        );
    }

    #[test]
    fn a_refusal_carries_codexs_reason() {
        let dir = tempfile::tempdir().unwrap();
        let exe = codex(dir.path(), "echo 'No active session found' >&2\nexit 1");
        let error = CodexQueue::new(exe, "thread-1".into())
            .send("hello")
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Codex would not take the message: No active session found"
        );
    }

    #[test]
    fn a_missing_codex_says_what_to_install() {
        let dir = tempfile::tempdir().unwrap();
        let error = CodexQueue::new(dir.path().join("absent"), "thread-1".into())
            .send("hello")
            .unwrap_err();
        assert!(error.to_string().contains("Install Codex CLI"));
    }
}
