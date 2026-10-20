use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::agent::infra::AgentAddress;
use crate::cmd::{ServerUseCaseFactory, ServerUseCases};
use crate::diff::infra::ScopeRequest;
use crate::error::Result;

use super::{Registry, ServerEntry};

/// Configuration survives requests; resolved Git windows do not.
pub(super) struct Session {
    factory: Arc<dyn ServerUseCaseFactory>,
    config: watch::Sender<SessionConfig>,
    identity: watch::Sender<ServerEntry>,
    registry: Arc<Registry>,
}

impl Session {
    pub fn new(
        factory: Arc<dyn ServerUseCaseFactory>,
        config: SessionConfig,
        identity: ServerEntry,
        registry: Arc<Registry>,
    ) -> Self {
        Self {
            factory,
            config: watch::channel(config).0,
            identity: watch::channel(identity).0,
            registry,
        }
    }

    pub fn open(&self) -> Result<ServerUseCases> {
        let config = self.config.borrow().clone();
        let cases = self.factory.build(config.scope, config.agent)?;
        self.record(&cases)?;
        Ok(cases)
    }

    /// The latest invocation wins, except where it knows less: a `serve` typed
    /// in a plain terminal has no session to report to, which is not the same as
    /// saying there is none, so the one already known is kept.
    pub fn configure(&self, mut config: SessionConfig) -> Result<ServerEntry> {
        if config.agent.is_none() {
            config.agent = self.config.borrow().agent.clone();
        }
        let cases = self
            .factory
            .build(config.scope.clone(), config.agent.clone())?;
        cases.review.execute()?;
        self.record(&cases)?;
        self.config.send_replace(config);
        Ok(self.identity())
    }

    pub fn identity(&self) -> ServerEntry {
        self.identity.borrow().clone()
    }

    pub fn changes(&self) -> watch::Receiver<SessionConfig> {
        self.config.subscribe()
    }

    fn record(&self, cases: &ServerUseCases) -> Result<()> {
        let scope = cases.scope.execute()?;
        let mut entry = self.identity();
        if entry.branch == scope.branch && entry.base == scope.base_ref {
            return Ok(());
        }
        entry.branch = scope.branch;
        entry.base = scope.base_ref;
        self.registry.register(&entry)?;
        self.identity.send_replace(entry);
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(flatten)]
    pub scope: ScopeRequest,
    pub watch: bool,
    /// The session the review reports to, from the invocation that knew one.
    #[serde(default)]
    pub agent: Option<AgentAddress>,
}
