//! Starting a server that outlives the terminal that started it.
//!
//! Running the same command again with one flag added, rather than forking:
//! the child re-resolves the window itself, so there is no state to carry
//! across a fork and nothing that has to agree between parent and child except
//! the flag below.
//!
//! The parent waits — but only until the server has bound a port and said so,
//! which is what lets it report a real port instead of a promise, and lets a
//! server that cannot start fail in the terminal like any other command.

use std::fs::File;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::registry::{Registry, ServerEntry};
use crate::error::{Error, Result};

/// The flag that means "you are the server". Named here because this is what
/// appends it; `serve` declares it, and a test holds the two together.
pub const FOREGROUND: &str = "--foreground";

/// How long the child gets to open the repository, read the map and bind. On a
/// large repository the first of those is the slow one.
const TO_START: Duration = Duration::from_secs(30);

const POLL: Duration = Duration::from_millis(20);

/// Run this same invocation in a process of its own, and return once it is
/// serving.
pub fn spawn(registry: &Registry) -> Result<ServerEntry> {
    let log = registry
        .logs_dir()
        .join(format!("starting-{}.log", std::process::id()));
    let out = File::create(&log)?;
    let err = out.try_clone()?;

    let mut child = Command::new(std::env::current_exe()?)
        .args(std::env::args_os().skip(1))
        .arg(FOREGROUND)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        // A process group of its own, so Ctrl-C in this terminal — and the
        // hangup when it closes — go to the shell and not to the review.
        .process_group(0)
        .spawn()?;

    let deadline = Instant::now() + TO_START;
    loop {
        if let Some(entry) = registry
            .running()?
            .into_iter()
            .find(|e| e.pid == child.id())
        {
            // The child holds this file open by descriptor, so renaming it
            // underneath is fine — and it puts the log where `stop` will know
            // to clear it away.
            std::fs::rename(&log, registry.log_file(entry.port))?;
            return Ok(entry);
        }

        // It ended without registering, which means it refused to start: no
        // map, a taken port, a detached HEAD. Whatever it said belongs in this
        // terminal, since that is where somebody is looking.
        if child.try_wait()?.is_some() {
            return Err(refused(&log));
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = std::fs::remove_file(&log);
            return Err(Error::msg(format!(
                "the server did not come up within {}s — run `farol serve {FOREGROUND}` to watch it try",
                TO_START.as_secs()
            )));
        }

        std::thread::sleep(POLL);
    }
}

/// What the child printed before giving up, handed back as this command's own
/// failure so the exit code and the message land where they would have without
/// the extra process.
fn refused(log: &std::path::Path) -> Error {
    let said = std::fs::read_to_string(log).unwrap_or_default();
    let _ = std::fs::remove_file(log);

    match said.trim() {
        "" => Error::msg("the server stopped without starting, and said nothing"),
        message => Error::msg(message.to_string()),
    }
}
