//! Telling the browser the map changed, without it having to ask.

use std::collections::HashMap;
use std::path::PathBuf;

use tokio::sync::broadcast;

/// Watch the git dir rather than the worktree: commits, checkouts and the
/// skill writing a map all land here, while a big working tree would flood us
/// with noise from builds.
pub(super) fn spawn(
    git_dir: PathBuf,
    common_dir: PathBuf,
    tx: broadcast::Sender<String>,
    config: tokio::sync::watch::Receiver<super::SessionConfig>,
    mut check_head: crate::diff::application::CheckHead,
) {
    std::thread::spawn(move || {
        use notify::{RecursiveMode, Watcher};

        // Linked worktree common dirs may contain ../..; notify reports the
        // resolved path, so both watch roots must use the same spelling.
        let roots = git_dir
            .canonicalize()
            .and_then(|git| common_dir.canonicalize().map(|common| (git, common)));
        let (git_dir, common_dir) = match roots {
            Ok(roots) => roots,
            Err(error) => {
                eprintln!("warning: cannot watch the repository: {error}");
                return;
            }
        };

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

        // Linked worktrees keep maps privately but branch refs in the common
        // directory. Its root also carries packed-refs; objects need no watch.
        if common_dir != git_dir {
            for (path, mode) in [
                (common_dir.clone(), RecursiveMode::NonRecursive),
                (common_dir.join("refs"), RecursiveMode::Recursive),
            ] {
                if let Err(e) = watcher.watch(&path, mode) {
                    eprintln!("warning: cannot watch {}: {e}", path.display());
                }
            }
        }

        if let Err(error) = check_head.execute() {
            eprintln!("warning: cannot check HEAD: {error}");
        }
        // The first map or comment event must get through, even at startup.
        let mut last: HashMap<&'static str, std::time::Instant> = HashMap::new();
        for event in raw_rx {
            if !config.borrow().watch {
                continue;
            }
            let Ok(event) = event else { continue };
            let Some(kind) = nudge_for(&event.paths, &git_dir, &common_dir) else {
                continue;
            };
            if kind == "head" {
                match check_head.execute() {
                    Ok(true) => {
                        let _ = tx.send(kind.to_string());
                    }
                    Ok(false) => {}
                    Err(error) => eprintln!("warning: cannot check HEAD: {error}"),
                }
                continue;
            }
            // Map and comment writes may produce several filesystem events.
            // Keep their quiet windows separate so neither silences the other.
            if last.get(kind).is_some_and(|t| t.elapsed() < QUIET) {
                continue;
            }
            last.insert(kind, std::time::Instant::now());
            let _ = tx.send(kind.to_string());
        }
    });
}

/// Coalesce map and comment writes; HEAD changes are deduplicated by identity.
const QUIET: std::time::Duration = std::time::Duration::from_millis(300);

/// What an event on the git dir means for the browser, if anything.
///
/// Most of what lands in the git dir is none of the reviewer's business —
/// objects being written, locks being taken. Three things change what is on
/// screen: a comment being written, the skill rewriting the map, and the branch
/// moving.
fn nudge_for(
    paths: &[PathBuf],
    git_dir: &std::path::Path,
    common_dir: &std::path::Path,
) -> Option<&'static str> {
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
    let touched_head = paths.iter().any(|p| {
        if p == &git_dir.join("HEAD") || p == &common_dir.join("packed-refs") {
            return true;
        }
        p.strip_prefix(common_dir.join("refs/heads"))
            .is_ok_and(|relative| {
                relative.components().next().is_some()
                    && !relative.as_os_str().to_string_lossy().ends_with(".lock")
            })
    });
    touched_head.then_some("head")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn(git_dir: PathBuf, tx: broadcast::Sender<String>) {
        super::spawn(
            git_dir.clone(),
            git_dir,
            tx,
            tokio::sync::watch::channel(super::super::SessionConfig {
                watch: true,
                ..Default::default()
            })
            .1,
            crate::diff::application::CheckHead::new(std::sync::Arc::new(
                crate::testing::FakeDiffSource::with_paths(&[]),
            )),
        );
    }

    fn nudge_for(paths: &[PathBuf]) -> Option<&'static str> {
        super::nudge_for(
            paths,
            std::path::Path::new("/repo/.git"),
            std::path::Path::new("/repo/.git"),
        )
    }

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
        assert_eq!(nudge_for(&paths(&["/repo/.git/packed-refs"])), Some("head"));
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
    fn a_fetch_or_push_does_not_ask_the_reader_to_refresh() {
        for path in [
            "/repo/.git/refs/remotes/origin/main",
            "/repo/.git/refs/tags/v1",
            "/repo/.git/FETCH_HEAD",
            "/repo/.git/HEAD.lock",
            "/repo/.git/refs/heads/main.lock",
            "/elsewhere/refs/heads/main",
            "/repo/.git/worktrees/other/HEAD",
        ] {
            assert_eq!(
                nudge_for(&paths(&[path])),
                None,
                "{path} does not signal a current branch change"
            );
        }
    }

    #[test]
    fn a_linked_worktree_watches_its_private_head_and_common_refs() {
        let git = std::path::Path::new("/repo/.git/worktrees/review");
        let common = std::path::Path::new("/repo/.git");
        for path in [
            "/repo/.git/worktrees/review/HEAD",
            "/repo/.git/refs/heads/review",
            "/repo/.git/packed-refs",
        ] {
            assert_eq!(super::nudge_for(&paths(&[path]), git, common), Some("head"));
        }
        assert_eq!(
            super::nudge_for(&paths(&["/repo/.git/HEAD"]), git, common),
            None
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
    fn watching_can_be_disabled_and_enabled_without_starting_another_server() {
        let dir = tempfile::tempdir().unwrap();
        let maps = dir.path().join("farol/x/maps");
        std::fs::create_dir_all(&maps).unwrap();
        let file = maps.join("head.json");
        let (config, updates) = tokio::sync::watch::channel(super::super::SessionConfig {
            watch: true,
            ..Default::default()
        });
        let (tx, mut rx) = broadcast::channel(16);
        super::spawn(
            dir.path().into(),
            dir.path().into(),
            tx,
            updates,
            crate::diff::application::CheckHead::new(std::sync::Arc::new(
                crate::testing::FakeDiffSource::with_paths(&[]),
            )),
        );
        assert_eq!(
            nudged_by(|| std::fs::write(&file, "{}").unwrap(), &mut rx).as_deref(),
            Some("map")
        );
        config.send_modify(|config| config.watch = false);
        std::thread::sleep(QUIET * 2);
        while rx.try_recv().is_ok() {}
        std::fs::write(&file, "disabled").unwrap();
        std::thread::sleep(QUIET * 2);
        assert!(rx.try_recv().is_err());
        config.send_modify(|config| config.watch = true);
        assert_eq!(
            nudged_by(|| std::fs::write(&file, "enabled").unwrap(), &mut rx).as_deref(),
            Some("map")
        );
    }

    #[test]
    fn news_of_one_kind_does_not_swallow_news_of_another() {
        // A comment write must not silence a map written immediately afterwards.
        let dir = tempfile::tempdir().unwrap();
        let maps = dir.path().join("farol").join("x").join("maps");
        std::fs::create_dir_all(&maps).unwrap();

        let (tx, mut rx) = broadcast::channel(16);
        spawn(dir.path().to_path_buf(), tx);

        let comments = dir.path().join("farol/x/comments");
        std::fs::create_dir_all(&comments).unwrap();
        let head = comments.join("note.md");
        assert_eq!(
            nudged_by(
                || std::fs::write(&head, "ref: refs/heads/x\n").unwrap(),
                &mut rx
            )
            .as_deref(),
            Some("comments"),
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
