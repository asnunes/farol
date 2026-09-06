use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

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
        let cases = self.factory.build(config.scope)?;
        self.record(&cases)?;
        Ok(cases)
    }

    pub fn configure(&self, config: SessionConfig) -> Result<ServerEntry> {
        let cases = self.factory.build(config.scope.clone())?;
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
}
