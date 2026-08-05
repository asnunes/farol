//! The composition root.
//!
//! The only place in the program that names a concrete implementation, and the
//! only place that assembles a use case out of services. A command receives use
//! cases; it never sees a service, a repository or a port.

use std::path::PathBuf;
use std::sync::Arc;

use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
use crate::diff::infra::{GixSource, ScopeRequest};
use crate::map::application::{
    AddBlock, AddFile, AddLineNote, AddSkim, CheckMap, DeriveMap, DiscardNote, GetFileDiff,
    GetReview, GetScope, MapReconciler, MapService, MoveBlock, RemoveBlock, RemoveFile,
    RemoveLineNote, RemoveSkim, ResetMap, RestoreNote, ShowMap, UpdateBlock, UpdateFile,
    UpdateLineNote,
};
use crate::map::infra::JsonMapRepository;
use crate::progress::application::{MarkViewed, ProgressStore, UnmarkViewed};
use crate::progress::infra::JsonProgressRepository;
use crate::shared::error::Result;
use crate::shared::paths::Workspace;

/// Every use case one invocation can reach.
///
/// A bag, deliberately: the CLI dispatches to all of them, so something has to
/// hold them. What matters is what is *not* here — no ports, no repositories,
/// no services. A command cannot reach past the operation it is performing.
pub struct Ctx {
    pub add_block: AddBlock,
    pub update_block: UpdateBlock,
    pub remove_block: RemoveBlock,
    pub move_block: MoveBlock,

    pub add_file: AddFile,
    pub update_file: UpdateFile,
    pub remove_file: RemoveFile,

    pub add_line_note: AddLineNote,
    pub update_line_note: UpdateLineNote,
    pub remove_line_note: RemoveLineNote,
    pub restore_note: RestoreNote,
    pub discard_note: DiscardNote,

    pub add_skim: AddSkim,
    pub remove_skim: RemoveSkim,

    pub derive_map: DeriveMap,
    pub show_map: ShowMap,
    pub check_map: CheckMap,
    pub reset_map: ResetMap,
    pub scope: GetScope,

    server: ServerUseCases,
    git_dir: PathBuf,
}

/// The subset a running server needs, kept together so `serve` hands over one
/// value instead of six.
#[derive(Clone)]
pub struct ServerUseCases {
    pub review: GetReview,
    pub file_diff: GetFileDiff,
    pub mark_viewed: MarkViewed,
    pub unmark_viewed: UnmarkViewed,
}

impl Ctx {
    /// Choose the real implementations for the repository farol was invoked in.
    pub fn from_workspace(request: ScopeRequest) -> Result<Self> {
        let workspace = Workspace::here()?;
        let git_dir = workspace.git_dir().to_path_buf();
        let branch = workspace.branch().to_string();

        let maps = Arc::new(JsonMapRepository::new(workspace.store()));
        let progress_repo = Arc::new(JsonProgressRepository::new(workspace.store()));
        let source = Arc::new(GixSource::open(workspace.into_repo(), &branch, &request)?);

        let scope = ReviewScope::new(source.clone());
        let diffs = FileDiffs::new(source.clone());
        let history = CommitHistory::new(source);

        let reconciler = MapReconciler::new(scope.clone(), diffs.clone());
        let map = MapService::new(scope.clone(), history, reconciler, maps);
        let progress = ProgressStore::new(progress_repo, diffs.clone());

        Ok(Self {
            add_block: AddBlock::new(map.clone()),
            update_block: UpdateBlock::new(map.clone()),
            remove_block: RemoveBlock::new(map.clone()),
            move_block: MoveBlock::new(map.clone()),

            add_file: AddFile::new(map.clone()),
            update_file: UpdateFile::new(map.clone()),
            remove_file: RemoveFile::new(map.clone()),

            add_line_note: AddLineNote::new(map.clone()),
            update_line_note: UpdateLineNote::new(map.clone()),
            remove_line_note: RemoveLineNote::new(map.clone()),
            restore_note: RestoreNote::new(map.clone()),
            discard_note: DiscardNote::new(map.clone()),

            add_skim: AddSkim::new(map.clone()),
            remove_skim: RemoveSkim::new(map.clone()),

            derive_map: DeriveMap::new(map.clone()),
            show_map: ShowMap::new(map.clone()),
            check_map: CheckMap::new(map.clone(), scope.clone()),
            reset_map: ResetMap::new(map.clone()),
            scope: GetScope::new(scope),

            server: ServerUseCases {
                review: GetReview::new(map, progress.clone()),
                file_diff: GetFileDiff::new(diffs),
                mark_viewed: MarkViewed::new(progress.clone()),
                unmark_viewed: UnmarkViewed::new(progress),
            },
            git_dir,
        })
    }

    pub fn server(&self) -> &ServerUseCases {
        &self.server
    }

    pub fn git_dir(&self) -> &PathBuf {
        &self.git_dir
    }
}
