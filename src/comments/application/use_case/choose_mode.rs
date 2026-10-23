use crate::comments::application::ReviewModes;
use crate::comments::domain::{ModeChoice, ReviewMode};
use crate::error::Result;

/// Set the review mode by hand, from `farol serve --peer-review` or the switch
/// on screen. It stays until changed, over whatever the pull request says.
#[derive(Clone)]
pub struct ChooseMode {
    modes: ReviewModes,
}

impl ChooseMode {
    pub fn new(modes: ReviewModes) -> Self {
        Self { modes }
    }

    pub fn execute(&self, mode: ReviewMode) -> Result<ModeChoice> {
        self.modes.choose(mode)
    }
}
