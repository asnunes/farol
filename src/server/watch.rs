//! Telling the browser the map changed, without it having to ask.

use std::collections::HashMap;
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

        // Empty, not "everything just now": nothing has been sent yet, so the
        // first change of each kind must get through. Starting the clock at
        // startup swallowed it if it landed within the quiet period, which is
        // exactly what happens when the skill finishes writing the map as the
        // server comes up.
        let mut last: HashMap<&'static str, std::time::Instant> = HashMap::new();
        for event in raw_rx {
            let Ok(event) = event else { continue };
            let Some(kind) = nudge_for(&event.paths) else {
                continue;
            };
            // Git rewrites several files per operation; one nudge is enough.
            // Per kind, though: `map derive` right after a commit writes the
            // map within milliseconds of the ref, and one quiet window for
            // both would let the commit swallow the map.
            if last.get(kind).is_some_and(|t| t.elapsed() < QUIET) {
                continue;
            }
            last.insert(kind, std::time::Instant::now());
            let _ = tx.send(kind.to_string());
        }
    });
}

/// How long to ignore further events of the same kind after nudging. A commit
/// rewrites `HEAD`, a ref and the index in quick succession; the browser needs
/// one reload, not three. Of the same kind only: different news is different
/// news however close together it lands.
const QUIET: std::time::Duration = std::time::Duration::from_millis(300);

/// What an event on the git dir means for the browser, if anything.
///
/// Most of what lands in the git dir is none of the reviewer's business —
/// objects being written, locks being taken. Three things change what is on
/// screen: a comment being written, the skill rewriting the map, and the branch
/// moving.
fn nudge_for(paths: &[PathBuf]) -> Option<&'static str> {
    let under = |p: &PathBuf, dir: &str| {
        let path = p.to_string_lossy();
        path.contains("/farol/") && path.contains(dir)
    };

    // Ahead of the map, and its own kind of news. A comment written from the
    // page itself lands here, and calling that "the map changed" would raise
    // the banner asking the reader to reload over something they just did.
    if paths.iter().any(|p| under(p, "/comments/")) {
        return Some("comments");
    }

    // Only the maps folder is the map. The store also holds `state.json`, which
    // is what the reader has marked as read, and that is written by the page on
    // every checkbox: reported as news it would raise the reload banner over
    // the reader's own click and then take it away again.
    if paths.iter().any(|p| under(p, "/maps/")) {
        return Some("map");
    }
    let touched_head = paths
        .iter()
        .any(|p| p.ends_with("HEAD") || p.to_string_lossy().contains("/refs/"));
    touched_head.then_some("head")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_skill_writing_a_map_reloads_the_screen() {
        assert_eq!(
            nudge_for(&paths(&["/repo/.git/farol/feature-x/maps/abc.json"])),
            Some("map")
        );
    }

    #[test]
    fn a_comment_being_written_is_its_own_news() {
        // The page writes these itself. Reported as a map change it would ask
        // the reader to reload over their own comment.
        assert_eq!(
            nudge_for(&paths(&[
                "/repo/.git/farol/feature-x/comments/18cb-3731.md"
            ])),
            Some("comments")
        );
    }

    #[test]
    fn what_the_reader_has_read_is_not_news_to_them() {
        // `state.json` is written by the page itself on every checkbox. Called
        // a map change, it raised the reload banner over the reader's own click
        // and dropped it a moment later, which read as a flicker.
        assert_eq!(
            nudge_for(&paths(&["/repo/.git/farol/feature-x/state.json"])),
            None
        );
    }

    #[test]
    fn the_branch_moving_reloads_the_screen() {
        // A commit or a checkout: the diff underneath the reviewer changed.
        assert_eq!(nudge_for(&paths(&["/repo/.git/HEAD"])), Some("head"));
        assert_eq!(
            nudge_for(&paths(&["/repo/.git/refs/heads/feature/x"])),
            Some("head")
        );
    }

    #[test]
    fn the_rest_of_the_git_dir_is_none_of_the_reviewers_business() {
        // Objects, locks and packs churn constantly; nudging on those would
        // reload the page while someone is reading.
        assert_eq!(nudge_for(&paths(&["/repo/.git/objects/ab/cdef"])), None);
        assert_eq!(nudge_for(&paths(&["/repo/.git/index.lock"])), None);
        assert_eq!(nudge_for(&paths(&["/repo/.git/COMMIT_EDITMSG"])), None);
    }

    #[test]
    fn a_map_written_in_the_same_breath_as_a_commit_reports_the_map() {
        // `map derive` writes the map and the branch may move around it. The
        // map is the more specific news, and the screen needs it either way.
        assert_eq!(
            nudge_for(&paths(&[
                "/repo/.git/HEAD",
                "/repo/.git/farol/x/maps/a.json"
            ])),
            Some("map")
        );
    }

    #[test]
    fn an_event_carrying_no_paths_says_nothing() {
        assert_eq!(nudge_for(&[]), None);
    }

    /// Touch the file until a nudge arrives, or give up.
    ///
    /// The watcher runs on a real thread and the platform takes its time to
    /// start delivering events. Sleeping "long enough" before writing once is a
    /// race that loses on a loaded machine — this one lost intermittently for a
    /// day before anyone caught it. Writing repeatedly cannot lose: either the
    /// watcher is up and the next touch is seen, or the deadline passes and the
    /// watcher genuinely never worked.
    fn nudged_by(touch: impl Fn(), rx: &mut broadcast::Receiver<String>) -> Option<String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            touch();
            let until = std::time::Instant::now() + std::time::Duration::from_millis(400);
            while std::time::Instant::now() < until {
                if let Ok(kind) = rx.try_recv() {
                    return Some(kind);
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        None
    }

    #[test]
    fn writing_a_map_nudges_the_browser() {
        let dir = tempfile::tempdir().unwrap();
        let maps = dir.path().join("farol").join("feature-x").join("maps");
        std::fs::create_dir_all(&maps).unwrap();

        let (tx, mut rx) = broadcast::channel(16);
        spawn(dir.path().to_path_buf(), tx);

        let file = maps.join("abc123.json");
        let kind = nudged_by(|| std::fs::write(&file, "{}").unwrap(), &mut rx);

        assert_eq!(kind.as_deref(), Some("map"));
    }

    #[test]
    fn news_of_one_kind_does_not_swallow_news_of_another() {
        // `map derive` right after a commit is the documented flow, and the
        // two writes land milliseconds apart. One quiet window for both let
        // the ref silence the map, so the page heard that the branch moved and
        // never that the map it is showing had been rewritten.
        let dir = tempfile::tempdir().unwrap();
        let maps = dir.path().join("farol").join("x").join("maps");
        std::fs::create_dir_all(&maps).unwrap();

        let (tx, mut rx) = broadcast::channel(16);
        spawn(dir.path().to_path_buf(), tx);

        let head = dir.path().join("HEAD");
        assert_eq!(
            nudged_by(
                || std::fs::write(&head, "ref: refs/heads/x\n").unwrap(),
                &mut rx
            )
            .as_deref(),
            Some("head"),
            "the watcher has to be awake, or the map below proves nothing"
        );

        let map = maps.join("abc123.json");
        assert_eq!(
            nudged_by(|| std::fs::write(&map, "{}").unwrap(), &mut rx).as_deref(),
            Some("map")
        );
    }

    #[test]
    fn churn_the_reviewer_does_not_care_about_is_left_alone() {
        // Objects and locks are written constantly; reloading on those would
        // pull the page out from under whoever is reading.
        let dir = tempfile::tempdir().unwrap();
        let objects = dir.path().join("objects").join("ab");
        let maps = dir.path().join("farol").join("x").join("maps");
        std::fs::create_dir_all(&objects).unwrap();
        std::fs::create_dir_all(&maps).unwrap();

        let (tx, mut rx) = broadcast::channel(16);
        spawn(dir.path().to_path_buf(), tx);

        // Prove the watcher is awake first, or the silence below would be the
        // silence of a watcher that never started — and the test would pass
        // for the one reason that makes it worthless.
        let map = maps.join("abc123.json");
        assert_eq!(
            nudged_by(|| std::fs::write(&map, "{}").unwrap(), &mut rx).as_deref(),
            Some("map"),
            "the watcher never came up, so this test proves nothing"
        );
        // Past the quiet period, so a second nudge would be allowed through.
        std::thread::sleep(QUIET * 2);

        std::fs::write(objects.join("cdef01"), "an object").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));

        assert!(rx.try_recv().is_err(), "nothing should have been sent");
    }

    #[test]
    fn a_directory_that_cannot_be_watched_gives_up_quietly() {
        // No panic, no channel closed early: farol still serves what it has.
        let (tx, mut rx) = broadcast::channel(16);

        spawn(PathBuf::from("/definitely/not/here"), tx);
        std::thread::sleep(std::time::Duration::from_millis(300));

        assert!(rx.try_recv().is_err());
    }
}
