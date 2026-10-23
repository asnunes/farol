//! Self or peer review, resolved and remembered.

use std::sync::Arc;

use crate::comments::domain::{ModeChoice, ModeStore, Readiness, ReviewMode, StoredMode};
use crate::error::Result;

/// The review mode for the branch.
///
/// Two ways in, because two kinds of caller need it. The page asks while it
/// is already asking the pull request, and gets the mode detected from it —
/// which is then written down. A message to the agent asks offline and gets
/// what was written down, so a send never waits on the network and the rule
/// it carries does not flip when the network blinks.
#[derive(Clone)]
pub struct ReviewModes {
    store: Arc<dyn ModeStore>,
}

impl ReviewModes {
    pub fn new(store: Arc<dyn ModeStore>) -> Self {
        Self { store }
    }

    /// What was chosen, or what the pull request said last time.
    pub fn current(&self) -> Result<ModeChoice> {
        Ok(ModeChoice::resolve(self.store.read()?.as_ref(), None))
    }

    /// The mode, given what the host just said, remembered when it taught
    /// farol something.
    pub fn detect(&self, readiness: &Readiness) -> Result<ModeChoice> {
        let stored = self.store.read()?;
        let choice = ModeChoice::resolve(stored.as_ref(), Some(readiness));
        if let Some(learned) = choice
            .to_store()
            .filter(|learned| Some(learned) != stored.as_ref())
        {
            self.store.write(&learned)?;
        }
        Ok(choice)
    }

    pub fn choose(&self, mode: ReviewMode) -> Result<ModeChoice> {
        self.store.write(&StoredMode {
            mode,
            chosen: true,
            author: None,
        })?;
        self.current()
    }
}
