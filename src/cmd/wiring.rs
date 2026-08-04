//! The composition root.
//!
//! This is the only place in the program that names a concrete implementation.
//! Everything below it receives its collaborators as trait objects, so any of
//! them can be replaced by a fake in a test without the code under test knowing.

use std::path::PathBuf;
use std::sync::Arc;

use crate::diff::domain::DiffSource;
use crate::diff::infra::{GixSource, ScopeRequest};
use crate::map::application::MapSession;
use crate::map::domain::MapRepository;
use crate::map::infra::JsonMapRepository;
use crate::progress::application::ProgressService;
use crate::progress::domain::ProgressRepository;
use crate::progress::infra::JsonProgressRepository;
use crate::shared::error::Result;
use crate::shared::paths::Workspace;

/// Everything a command works against, already resolved.
///
/// Constructed only through [`Ctx::new`], which takes its collaborators rather
/// than building them — [`Ctx::from_workspace`] is the wiring that does the
/// choosing, and it is deliberately the only such place.
pub struct Ctx {
    source: Arc<dyn DiffSource>,
    maps: Arc<dyn MapRepository>,
    progress: Arc<dyn ProgressRepository>,
    git_dir: PathBuf,
}

impl Ctx {
    pub fn new(
        source: Arc<dyn DiffSource>,
        maps: Arc<dyn MapRepository>,
        progress: Arc<dyn ProgressRepository>,
        git_dir: PathBuf,
    ) -> Self {
        Self {
            source,
            maps,
            progress,
            git_dir,
        }
    }

    /// Wire the real implementations for the repository farol was invoked in.
    pub fn from_workspace(request: ScopeRequest) -> Result<Self> {
        let workspace = Workspace::here()?;
        let git_dir = workspace.git_dir().to_path_buf();
        let maps = Arc::new(JsonMapRepository::new(workspace.store()));
        let progress = Arc::new(JsonProgressRepository::new(workspace.store()));
        let branch = workspace.branch().to_string();
        let source = Arc::new(GixSource::open(workspace.into_repo(), &branch, &request)?);

        Ok(Self::new(source, maps, progress, git_dir))
    }

    pub fn source(&self) -> &dyn DiffSource {
        self.source.as_ref()
    }

    pub fn source_arc(&self) -> Arc<dyn DiffSource> {
        Arc::clone(&self.source)
    }

    pub fn progress_arc(&self) -> Arc<dyn ProgressRepository> {
        Arc::clone(&self.progress)
    }

    pub fn git_dir(&self) -> &PathBuf {
        &self.git_dir
    }

    pub fn map(&self) -> MapSession<'_> {
        MapSession::new(self.source.as_ref(), self.maps.as_ref())
    }

    pub fn progress(&self) -> ProgressService<'_> {
        ProgressService::new(self.progress.as_ref(), self.source.as_ref())
    }
}
