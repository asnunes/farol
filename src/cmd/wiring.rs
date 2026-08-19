//! The composition root.
//!
//! The only place in the program that names a concrete implementation, and the
//! only place that assembles a use case out of services. A command receives use
//! cases; it never sees a service, a repository or a port.

use std::path::PathBuf;
use std::sync::Arc;

use crate::comments::application::{Comments, PublishReview, ReviewReadiness, SaveToken};
use crate::comments::domain::ReviewPublisher;
use crate::comments::infra::{GitHub, MarkdownComments, TokenFile, Unhosted};
use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
use crate::diff::infra::{GixSource, ScopeRequest};
use crate::error::Result;
use crate::map::application::{
    AddBlock, AddFile, AddLineNote, AddSkim, CheckMap, DeriveMap, DiscardNote, ExportMap,
    GetFileDiff, GetReview, GetScope, ImportMap, MapDerivation, MapEditor, MapReconciler,
    MapVersions, MoveBlock, RemoveBlock, RemoveFile, RemoveLineNote, RemoveSkim, ResetMap,
    RestoreNote, ShowMap, UpdateBlock, UpdateFile, UpdateLineNote,
};
use crate::map::infra::JsonMapRepository;
use crate::progress::application::{MarkViewed, ProgressStore, UnmarkViewed};
use crate::progress::infra::JsonProgressRepository;
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
    pub export_map: ExportMap,
    pub import_map: ImportMap,
    pub scope: GetScope,

    pub comments: Comments,
    pub readiness: ReviewReadiness,
    pub publish_review: Arc<PublishReview>,

    server: ServerUseCases,
    git_dir: PathBuf,
    root: PathBuf,
}

/// The subset a running server needs, kept together so `serve` hands over one
/// value instead of six.
#[derive(Clone)]
pub struct ServerUseCases {
    pub review: GetReview,
    pub file_diff: GetFileDiff,
    pub mark_viewed: MarkViewed,
    pub unmark_viewed: UnmarkViewed,
    pub comments: Comments,
    pub readiness: ReviewReadiness,
    pub publish_review: Arc<PublishReview>,
    pub save_token: Arc<SaveToken>,
}

impl Ctx {
    /// Choose the real implementations for the repository farol was invoked in.
    pub fn from_workspace(request: ScopeRequest) -> Result<Self> {
        let workspace = Workspace::here()?;
        let git_dir = workspace.git_dir().to_path_buf();
        let root = workspace.root();
        let branch = workspace.branch().to_string();
        // Both read before the repository is handed to the diff source, which
        // consumes it.
        let name = workspace.name();
        let remote = workspace.remote();

        let comment_store = Arc::new(MarkdownComments::new(&workspace.store(), &root));
        let maps = Arc::new(JsonMapRepository::new(workspace.store()));
        let maps_for_sharing = maps.clone();
        let progress_repo = Arc::new(JsonProgressRepository::new(workspace.store()));
        let source = Arc::new(GixSource::open(workspace.into_repo(), &branch, &request)?);

        let scope = ReviewScope::new(source.clone());
        let diffs = FileDiffs::new(source.clone());
        let history = CommitHistory::new(source);

        let credentials = Arc::new(TokenFile::here()?);
        let publisher: Arc<dyn ReviewPublisher> = match remote {
            Some(remote) => Arc::new(GitHub::new(remote, credentials.clone())),
            None => Arc::new(Unhosted),
        };

        let comments = Comments::new(
            comment_store.clone(),
            GetScope::new(scope.clone()),
            diffs.clone(),
        );
        let reconciler = MapReconciler::new(scope.clone(), diffs.clone());
        let versions = MapVersions::new(scope.clone(), history.clone(), maps.clone());
        let derivation =
            MapDerivation::new(versions.clone(), scope.clone(), reconciler, maps.clone());
        let editor = MapEditor::new(derivation.clone(), versions.clone(), maps);
        let progress = ProgressStore::new(progress_repo, diffs.clone());
        let scope_for_publishing = scope.clone();
        let diffs_for_publishing = diffs.clone();

        let readiness = ReviewReadiness::new(publisher.clone(), scope_for_publishing.clone());
        let publish_review = Arc::new(PublishReview::new(
            comment_store,
            publisher,
            scope_for_publishing,
            diffs_for_publishing,
            history,
        ));

        Ok(Self {
            add_block: AddBlock::new(editor.clone()),
            update_block: UpdateBlock::new(editor.clone()),
            remove_block: RemoveBlock::new(editor.clone()),
            move_block: MoveBlock::new(editor.clone()),

            add_file: AddFile::new(editor.clone()),
            update_file: UpdateFile::new(editor.clone()),
            remove_file: RemoveFile::new(editor.clone()),

            add_line_note: AddLineNote::new(editor.clone()),
            update_line_note: UpdateLineNote::new(editor.clone()),
            remove_line_note: RemoveLineNote::new(editor.clone()),
            restore_note: RestoreNote::new(editor.clone()),
            discard_note: DiscardNote::new(editor.clone()),

            add_skim: AddSkim::new(editor.clone()),
            remove_skim: RemoveSkim::new(editor.clone()),

            derive_map: DeriveMap::new(derivation, versions.clone()),
            show_map: ShowMap::new(versions.clone()),
            check_map: CheckMap::new(versions.clone(), scope.clone()),
            reset_map: ResetMap::new(editor),
            export_map: ExportMap::new(versions.clone(), scope.clone(), name),
            import_map: ImportMap::new(scope.clone(), maps_for_sharing),
            scope: GetScope::new(scope.clone()),

            comments: comments.clone(),

            // Built once and shared: the terminal and the browser ask the same
            // two questions of the same host, and a second instance would be a
            // second agent and a second connection pool for no reason.
            readiness: readiness.clone(),
            publish_review: publish_review.clone(),

            server: ServerUseCases {
                review: GetReview::new(versions, scope, progress.clone()),
                file_diff: GetFileDiff::new(diffs),
                mark_viewed: MarkViewed::new(progress.clone()),
                unmark_viewed: UnmarkViewed::new(progress),
                comments,
                readiness,
                publish_review,
                save_token: Arc::new(SaveToken::new(credentials)),
            },
            git_dir,
            root,
        })
    }

    pub fn server(&self) -> &ServerUseCases {
        &self.server
    }

    pub fn git_dir(&self) -> &PathBuf {
        &self.git_dir
    }

    /// The working tree the review is of.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }
}
