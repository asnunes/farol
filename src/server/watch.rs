//! Telling the browser the map changed, without it having to ask.

use std::path::PathBuf;

use tokio::sync::broadcast;

/// Watch the git dir rather than the worktree: commits, checkouts and the
/// skill writing a map all land here, while a big working tree would flood us
/// with noise from builds.
pub(super) fn spawn(git_dir: PathBuf, tx: broadcast::Sender<String>) {
    std::thread::spawn(move || {
        use notify::{RecursiveMode, Watcher};

        let (raw_tx, raw_rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(raw_tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("warning: cannot watch the repository: {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&git_dir, RecursiveMode::Recursive) {
            eprintln!("warning: cannot watch {}: {e}", git_dir.display());
            return;
        }

        let mut last = std::time::Instant::now();
        for event in raw_rx {
            let Ok(event) = event else { continue };
            let touched_map = event
                .paths
                .iter()
                .any(|p| p.to_string_lossy().contains("/farol/"));
            let touched_head = event
                .paths
                .iter()
                .any(|p| p.ends_with("HEAD") || p.to_string_lossy().contains("/refs/"));

            if !touched_map && !touched_head {
                continue;
            }
            // Git rewrites several files per operation; one nudge is enough.
            if last.elapsed() < std::time::Duration::from_millis(300) {
                continue;
            }
            last = std::time::Instant::now();
            let kind = if touched_map { "map" } else { "head" };
            let _ = tx.send(kind.to_string());
        }
    });
}
