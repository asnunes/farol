use std::sync::Arc;

use crate::agent::domain::{AgentChannel, AgentError};
use crate::comments::application::{ReviewModes, message};
use crate::config::domain::ConfigRepository;
use crate::diff::application::ReviewScope;
use crate::error::Result;

/// Ask the agent for the pull request's title and description, following the
/// instruction from Settings. It writes them back with `farol github draft`,
/// and the page fills the form from there; nothing is opened until the
/// reviewer presses Create.
#[derive(Clone)]
pub struct GeneratePullDraft {
    channel: Option<Arc<dyn AgentChannel>>,
    config: Arc<dyn ConfigRepository>,
    modes: ReviewModes,
    scope: ReviewScope,
}

impl GeneratePullDraft {
    pub fn new(
        channel: Option<Arc<dyn AgentChannel>>,
        config: Arc<dyn ConfigRepository>,
        modes: ReviewModes,
        scope: ReviewScope,
    ) -> Self {
        Self {
            channel,
            config,
            modes,
            scope,
        }
    }

    pub fn execute(&self) -> Result<()> {
        let channel = self.channel.as_ref().ok_or(AgentError::NoSession)?;
        let scope = self.scope.get()?;
        let config = self.config.read()?;
        let mode = self.modes.current()?.mode;
        channel.send(&message::pull_draft(
            &scope.branch,
            &scope.base_ref,
            mode,
            config.pull_request_instruction(),
        ))?;
        Ok(())
    }
}
