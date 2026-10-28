use std::sync::Arc;

use crate::error::Result;
use crate::lenses::domain::{LensAnswerRepository, LensError, LensId, LensTarget};
use crate::progress::application::ProgressStore;

/// Mark a lens as seen by the reviewer, or take the mark back.
///
/// Kept with the reading progress, which the server writes, and pinned to the
/// answer as it is: a lens the session rewrites asks to be read again.
#[derive(Clone)]
pub struct SeeLens {
    answers: Arc<dyn LensAnswerRepository>,
    progress: ProgressStore,
}

impl SeeLens {
    pub fn new(answers: Arc<dyn LensAnswerRepository>, progress: ProgressStore) -> Self {
        Self { answers, progress }
    }

    pub fn execute(&self, id: &LensId, target: &LensTarget, seen: bool, at: &str) -> Result<()> {
        let answer = self
            .answers
            .find(id, target)?
            .filter(|answer| &answer.target == target)
            .ok_or_else(|| LensError::NothingToKeep { id: id.to_string() })?;
        let fingerprint = answer.fingerprint();
        self.progress
            .see_lens(&answer.seen_key(), seen.then_some(fingerprint.as_str()), at)?;
        Ok(())
    }
}
