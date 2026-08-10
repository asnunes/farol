use crate::error::Result;
use crate::progress::domain::{PROGRESS_VERSION, Progress, ProgressRepository};
use crate::shared::paths::Store;

pub struct JsonProgressRepository {
    store: Store,
}

impl JsonProgressRepository {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl ProgressRepository for JsonProgressRepository {
    fn load(&self) -> Result<Progress> {
        let Ok(raw) = std::fs::read_to_string(self.store.state_file()) else {
            return Ok(Progress::new());
        };
        match serde_json::from_str::<Progress>(&raw) {
            Ok(p) if p.version == PROGRESS_VERSION => Ok(p),
            // Losing read-state is an inconvenience, not a disaster: worst case
            // the reviewer re-ticks what they already read.
            _ => Ok(Progress::new()),
        }
    }

    fn save(&self, progress: &Progress) -> Result<()> {
        self.store.write_json(&self.store.state_file(), progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::domain::ProgressRepository;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path(), "feature/x");
        (dir, store)
    }

    #[test]
    fn a_reviewer_who_has_read_nothing_starts_from_empty() {
        // Nothing on disk yet is the ordinary first run, not a failure.
        let (_dir, store) = store();

        let progress = JsonProgressRepository::new(store).load().unwrap();

        assert!(progress.viewed.is_empty());
        assert_eq!(progress.version, PROGRESS_VERSION);
    }

    #[test]
    fn what_was_read_survives_being_written_and_read_back() {
        let (_dir, store) = store();
        let repo = JsonProgressRepository::new(store);

        let mut progress = Progress::new();
        progress.mark("src/a.rs", "hash-1", "2026-08-06T12:00:00Z");
        repo.save(&progress).unwrap();

        let back = repo.load().unwrap();
        assert!(back.is_current("src/a.rs", "hash-1"));
        assert_eq!(back.viewed[0].at, "2026-08-06T12:00:00Z");
    }

    #[test]
    fn a_second_save_replaces_the_first_rather_than_appending() {
        let (_dir, store) = store();
        let repo = JsonProgressRepository::new(store);

        let mut progress = Progress::new();
        progress.mark("src/a.rs", "hash-1", "now");
        repo.save(&progress).unwrap();
        progress.unmark("src/a.rs");
        repo.save(&progress).unwrap();

        assert!(repo.load().unwrap().viewed.is_empty());
    }

    #[test]
    fn a_file_from_a_future_version_is_discarded_instead_of_misread() {
        // The shape may have changed under it. Re-ticking what you read is an
        // inconvenience; a wrong tick is a file the reviewer never looks at.
        let (_dir, store) = store();
        store
            .write_json(
                &store.state_file(),
                &serde_json::json!({
                    "version": PROGRESS_VERSION + 1,
                    "viewed": [{ "path": "src/a.rs", "content_hash": "h", "at": "now" }],
                }),
            )
            .unwrap();

        assert!(
            JsonProgressRepository::new(store)
                .load()
                .unwrap()
                .viewed
                .is_empty()
        );
    }

    #[test]
    fn a_truncated_file_does_not_take_the_server_down_with_it() {
        // A crash mid-write, or a file cut short by a full disk.
        let (_dir, store) = store();
        store.ensure().unwrap();
        std::fs::write(store.state_file(), "{\"version\": 1, \"viewed\": [{").unwrap();

        let progress = JsonProgressRepository::new(store).load().unwrap();

        assert!(progress.viewed.is_empty());
    }

    #[test]
    fn two_branches_do_not_read_each_others_progress() {
        // The store is keyed by branch; reviewing one must not mark the other.
        let dir = tempfile::tempdir().unwrap();
        let mine = JsonProgressRepository::new(Store::new(dir.path(), "feature/x"));
        let theirs = JsonProgressRepository::new(Store::new(dir.path(), "feature/y"));

        let mut progress = Progress::new();
        progress.mark("src/a.rs", "hash-1", "now");
        mine.save(&progress).unwrap();

        assert!(theirs.load().unwrap().viewed.is_empty());
    }
}
