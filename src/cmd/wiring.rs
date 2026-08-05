//! The composition root.
//!
//! This is the only place in the program that names a concrete implementation.
//! Everything below it receives services, not ports: an entry point never sees
//! `DiffSource` or `MapRepository`, only what it can *do*.

use std::path::PathBuf;
use std::sync::Arc;

use crate::diff::application::DiffService;
use crate::diff::infra::{GixSource, ScopeRequest};
use crate::map::application::MapSession;
use crate::map::infra::JsonMapRepository;
use crate::progress::application::ProgressService;
use crate::progress::infra::JsonProgressRepository;
use crate::shared::error::Result;
use crate::shared::paths::Workspace;

/// The use cases available to one invocation.
///
/// Holds services rather than ports on purpose: a command asks for what it can
/// do, not for the plumbing underneath. Only `serve` needs more, because it
/// hands the same services to a long-lived server.
pub struct Ctx {
    map: MapSession,
    progress: ProgressService,
    git_dir: PathBuf,
}

impl Ctx {
    pub fn new(map: MapSession, progress: ProgressService, git_dir: PathBuf) -> Self {
        Self {
            map,
            progress,
            git_dir,
        }
    }

    /// Choose the real implementations for the repository farol was invoked in.
    /// The only function in the program that names a concrete type.
    pub fn from_workspace(request: ScopeRequest) -> Result<Self> {
        let workspace = Workspace::here()?;
        let git_dir = workspace.git_dir().to_path_buf();
        let branch = workspace.branch().to_string();

        let maps = Arc::new(JsonMapRepository::new(workspace.store()));
        let progress_repo = Arc::new(JsonProgressRepository::new(workspace.store()));
        let source = Arc::new(GixSource::open(workspace.into_repo(), &branch, &request)?);

        let diff = DiffService::new(source);
        let map = MapSession::new(diff.clone(), maps);
        let progress = ProgressService::new(progress_repo, diff);

        Ok(Self::new(map, progress, git_dir))
    }

    pub fn map(&self) -> &MapSession {
        &self.map
    }

    pub fn progress(&self) -> &ProgressService {
        &self.progress
    }

    pub fn git_dir(&self) -> &PathBuf {
        &self.git_dir
    }
}
