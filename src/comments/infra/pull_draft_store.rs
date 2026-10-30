use crate::comments::domain::{PullDraft, PullDraftStore};
use crate::error::Result;
use crate::shared::paths::Store;

/// `pull-request/draft.json` beside the branch's other state.
pub struct JsonPullDraft {
    store: Store,
}

impl JsonPullDraft {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl PullDraftStore for JsonPullDraft {
    fn read(&self) -> Result<Option<PullDraft>> {
        Ok(self.store.read_json(&self.store.pull_draft_file(), |_| {}))
    }

    fn write(&self, draft: &PullDraft) -> Result<()> {
        let file = self.store.pull_draft_file();
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        self.store.write_json(&file, draft)
    }

    fn clear(&self) -> Result<()> {
        match std::fs::remove_file(self.store.pull_draft_file()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_survives_being_written_and_goes_once_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = JsonPullDraft::new(Store::new(dir.path(), "feature/x"));
        assert_eq!(drafts.read().unwrap(), None);

        let draft = PullDraft {
            title: "Stop a review from the page".into(),
            body: "A Stop button.".into(),
        };
        drafts.write(&draft).unwrap();
        assert_eq!(drafts.read().unwrap(), Some(draft));

        drafts.clear().unwrap();
        assert_eq!(drafts.read().unwrap(), None);
        drafts.clear().unwrap();
    }
}
