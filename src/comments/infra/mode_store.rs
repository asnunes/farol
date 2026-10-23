use crate::comments::domain::{ModeStore, StoredMode};
use crate::error::Result;
use crate::shared::paths::Store;

/// `review-mode.json` beside the branch's other state.
pub struct JsonModeStore {
    store: Store,
}

impl JsonModeStore {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl ModeStore for JsonModeStore {
    /// A file that no longer parses is no choice at all: the mode is detected
    /// again, which costs one look at the pull request.
    fn read(&self) -> Result<Option<StoredMode>> {
        Ok(self.store.read_json(&self.store.mode_file(), |_| {}))
    }

    fn write(&self, mode: &StoredMode) -> Result<()> {
        self.store.write_json(&self.store.mode_file(), mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::ReviewMode;

    #[test]
    fn a_choice_survives_being_written_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let modes = JsonModeStore::new(Store::new(dir.path(), "feature/x"));
        assert_eq!(modes.read().unwrap(), None);

        let chosen = StoredMode {
            mode: ReviewMode::PeerReview,
            chosen: true,
            author: None,
        };
        modes.write(&chosen).unwrap();

        assert_eq!(modes.read().unwrap(), Some(chosen));
    }
}
