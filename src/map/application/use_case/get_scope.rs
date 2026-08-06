use crate::diff::application::ReviewScope;
use crate::diff::domain::{ReviewPath, Scope};
use crate::shared::error::Result;

/// The files under review, as farol resolved them.
#[derive(Clone)]
pub struct GetScope {
    scope: ReviewScope,
}

impl GetScope {
    pub fn new(scope: ReviewScope) -> Self {
        Self { scope }
    }

    pub fn execute(&self) -> Result<Scope> {
        self.scope.get().cloned()
    }

    /// Turning a raw path into a proven one is part of reading the scope, so it
    /// lives with it rather than in every use case that needs a path.
    pub fn path(&self, raw: &str) -> Result<ReviewPath> {
        self.scope.path(raw)
    }

    pub fn paths(&self, raw: &[String]) -> Result<Vec<ReviewPath>> {
        self.scope.paths(raw)
    }
}
