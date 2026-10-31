//! The composition root.
//!
//! The only place in the program that names a concrete implementation, and the
//! only place that assembles a use case out of services. A command receives use
//! cases; it never sees a service, a repository or a port.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::agent::domain::AgentChannel;
use crate::agent::infra::{AgentAddress, ClaudeSocket, CodexQueue};
use crate::comments::application::{
    ChooseMode, CloseThread, CommentReach, Comments, EditPending, GeneratePending,
    GeneratePullDraft, OpenPullRequest, PublishReview, PullRequestGate, RemovePending, ReviewModes,
    ReviewReadiness, SendComment, SyncReadMarks, WritePending, WritePullDraft,
};
use crate::comments::domain::{BranchPusher, PullDraftStore, ReviewPublisher};
use crate::comments::infra::{
    GhCredentials, GitHub, GitPush, JsonModeStore, JsonPullDraft, Logins, MarkdownComments,
    Unhosted,
};
use crate::config::domain::ConfigRepository;
use crate::config::infra::JsonConfigRepository;
use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
use crate::diff::infra::{GixSource, ScopeRequest};
use crate::error::Result;
use crate::lenses::application::{
    AddLens, DeleteLens, DisableLens, EnableLens, GetReviewWithLenses, KeepAnswer, ListLenses,
    ListReadFiles, PendingAnswers, SeeLens, ShowAnswer, UpdateLens, WriteAnswer,
};
use crate::lenses::domain::LensCatalogRepository;
use crate::lenses::infra::{JsonLensAnswers, JsonLensCatalog};
use crate::map::application::{
    AddBlock, AddFile, AddLineNote, CheckMap, DeriveMap, DiscardNote, ExportMap, FocusFile,
    GetFileDiff, GetFileLines, GetReview, GetScope, ImportMap, MapDerivation, MapEditor,
    MapReconciler, MapVersions, MoveBlock, RemoveBlock, RemoveFile, RemoveLineNote, ResetMap,
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

    pub focus_file: FocusFile,

    pub derive_map: DeriveMap,
    pub show_map: ShowMap,
    pub check_map: CheckMap,
    pub reset_map: ResetMap,
    pub export_map: ExportMap,
    pub import_map: ImportMap,
    pub scope: GetScope,

    pub comments: Comments,
    pub close_thread: CloseThread,
    pub write_pending: WritePending,
    pub edit_pending: EditPending,
    pub remove_pending: RemovePending,
    pub choose_mode: ChooseMode,
    pub readiness: ReviewReadiness,
    pub publish_review: PublishReview,
    pub sync_read_marks: SyncReadMarks,
    pub write_pull_draft: WritePullDraft,

    pub write_answer: WriteAnswer,
    pub keep_answer: KeepAnswer,
    pub pending_answers: PendingAnswers,
    pub show_answer: ShowAnswer,

    /// Only when serving: the CLI never sends to an agent, and building this
    /// would make every command resolve the configuration folder for nothing.
    server: Option<ServerUseCases>,
    git_dir: PathBuf,
    root: PathBuf,
}

/// The subset a running server needs, kept together so `serve` hands over one
/// value instead of six.
#[derive(Clone)]
pub struct ServerUseCases {
    pub scope: GetScope,
    pub review: GetReviewWithLenses,
    pub see_lens: SeeLens,
    pub file_diff: GetFileDiff,
    pub file_lines: GetFileLines,
    pub mark_viewed: MarkViewed,
    pub unmark_viewed: UnmarkViewed,
    pub comments: Comments,
    pub close_thread: CloseThread,
    pub send_comment: SendComment,
    pub generate_pending: GeneratePending,
    pub write_pending: WritePending,
    pub edit_pending: EditPending,
    pub remove_pending: RemovePending,
    pub choose_mode: ChooseMode,
    pub readiness: ReviewReadiness,
    pub publish_review: PublishReview,
    pub sync_read_marks: SyncReadMarks,
    pub open_pull_request: OpenPullRequest,
    pub generate_pull_draft: GeneratePullDraft,
}

/// What a server adds to a context: the session it reports to, and where the
/// user's preferences are read from.
pub struct Serving {
    pub agent: Option<AgentAddress>,
    pub config: Arc<dyn ConfigRepository>,
    /// Who the GitHub token belongs to, kept across requests.
    pub logins: Arc<Logins>,
}

impl Ctx {
    /// Choose the real implementations for the repository farol was invoked in.
    pub fn from_workspace(request: ScopeRequest) -> Result<Self> {
        Self::at(&std::env::current_dir()?, request)
    }

    pub fn at(root: &Path, request: ScopeRequest) -> Result<Self> {
        Self::assemble(root, request, None)
    }

    fn assemble(root: &Path, request: ScopeRequest, serving: Option<Serving>) -> Result<Self> {
        let workspace = Workspace::discover(root)?;
        let git_dir = workspace.git_dir().to_path_buf();
        let root = workspace.root();
        let branch = workspace.branch().to_string();
        // Both read before the repository is handed to the diff source, which
        // consumes it.
        let name = workspace.name();
        let remote = workspace.remote();

        let comment_store = Arc::new(MarkdownComments::new(&workspace.store(), &root));
        let pusher: Arc<dyn BranchPusher> = Arc::new(GitPush::new(workspace.root()));
        // Read here by the CLI too: the session writes answers from the terminal
        // and each one is checked against the user's lenses.
        let lens_catalog = lens_catalog()?;
        let lens_answers = Arc::new(JsonLensAnswers::new(workspace.store().lenses_dir()));
        let modes = ReviewModes::new(Arc::new(JsonModeStore::new(workspace.store())));
        let drafts: Arc<dyn PullDraftStore> = Arc::new(JsonPullDraft::new(workspace.store()));
        let maps = Arc::new(JsonMapRepository::new(workspace.store()));
        let maps_for_sharing = maps.clone();
        let progress_repo = Arc::new(JsonProgressRepository::new(workspace.store()));
        let source = Arc::new(GixSource::open(workspace.into_repo(), &branch, &request)?);

        let scope = ReviewScope::new(source.clone());
        let diffs = FileDiffs::new(source.clone());
        let history = CommitHistory::new(source);

        let host = remote.as_ref().map(|remote| remote.host.clone());
        let credentials = Arc::new(GhCredentials::new("gh"));
        let publisher: Arc<dyn ReviewPublisher> = match remote {
            Some(remote) => Arc::new(GitHub::new(
                remote,
                credentials.clone(),
                serving
                    .as_ref()
                    .map(|serving| serving.logins.clone())
                    .unwrap_or_default(),
            )),
            None => Arc::new(Unhosted),
        };

        let reach = CommentReach::new(scope.clone(), diffs.clone());
        let comments = Comments::new(
            comment_store.clone(),
            GetScope::new(scope.clone()),
            reach.clone(),
        );
        let reconciler = MapReconciler::new(scope.clone(), diffs.clone());
        let versions = MapVersions::new(scope.clone(), history.clone(), maps.clone());
        let derivation =
            MapDerivation::new(versions.clone(), scope.clone(), reconciler, maps.clone());
        let editor = MapEditor::new(derivation.clone(), versions.clone(), maps);
        let progress = ProgressStore::new(progress_repo, diffs.clone());
        let scope_for_publishing = scope.clone();
        let scope_for_lines = scope.clone();

        let review = GetReviewWithLenses::new(
            GetReview::new(versions.clone(), scope.clone(), progress.clone()),
            lens_catalog.clone(),
            lens_answers.clone(),
        );
        let read = ListReadFiles::new(progress.clone(), scope.clone(), review.clone());
        let gate = PullRequestGate::new(publisher.clone(), scope.clone(), history.clone());
        let readiness = ReviewReadiness::new(
            gate.clone(),
            modes.clone(),
            comments.clone(),
            publisher.clone(),
            host.clone(),
            read.clone(),
            drafts.clone(),
        );
        let sync_read_marks = SyncReadMarks::new(gate.clone(), publisher.clone(), read.clone());
        let publish_review = PublishReview::new(
            comments.clone(),
            gate.clone(),
            publisher.clone(),
            scope_for_publishing,
            read,
        );
        let open_pull_request = OpenPullRequest::new(
            gate.clone(),
            publisher.clone(),
            pusher,
            scope.clone(),
            drafts.clone(),
        );
        let close_thread = CloseThread::new(comments.clone());
        let write_pending = WritePending::new(comments.clone(), gate, publisher.clone());
        let edit_pending = EditPending::new(publisher.clone());
        let remove_pending = RemovePending::new(comments.clone(), publisher.clone());
        let choose_mode = ChooseMode::new(modes.clone());
        let write_answer = WriteAnswer::new(
            lens_catalog.clone(),
            lens_answers.clone(),
            scope.clone(),
            diffs.clone(),
            versions.clone(),
        );
        let pending_answers = PendingAnswers::new(
            lens_catalog.clone(),
            lens_answers.clone(),
            scope.clone(),
            versions.clone(),
        );

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

            focus_file: FocusFile::new(editor.clone()),

            derive_map: DeriveMap::new(derivation, versions.clone()),
            show_map: ShowMap::new(versions.clone()),
            check_map: CheckMap::new(versions.clone(), scope.clone()),
            reset_map: ResetMap::new(editor),
            export_map: ExportMap::new(versions.clone(), scope.clone(), name),
            import_map: ImportMap::new(scope.clone(), maps_for_sharing),
            scope: GetScope::new(scope.clone()),

            comments: comments.clone(),
            close_thread: close_thread.clone(),
            write_pending: write_pending.clone(),
            edit_pending: edit_pending.clone(),
            remove_pending: remove_pending.clone(),
            choose_mode: choose_mode.clone(),
            readiness: readiness.clone(),
            publish_review: publish_review.clone(),
            sync_read_marks: sync_read_marks.clone(),
            write_pull_draft: WritePullDraft::new(drafts),

            keep_answer: KeepAnswer::new(lens_answers.clone(), write_answer.clone()),
            write_answer,
            pending_answers,
            show_answer: ShowAnswer::new(lens_answers.clone()),

            server: serving.map(|serving| ServerUseCases {
                scope: GetScope::new(scope.clone()),
                see_lens: SeeLens::new(lens_answers.clone(), progress.clone()),
                review,
                file_diff: GetFileDiff::new(diffs),
                file_lines: GetFileLines::new(scope_for_lines),
                mark_viewed: MarkViewed::new(progress.clone()),
                unmark_viewed: UnmarkViewed::new(progress),
                send_comment: SendComment::new(
                    comments.clone(),
                    serving.agent.as_ref().map(channel),
                    serving.config.clone(),
                    modes.clone(),
                ),
                generate_pull_draft: GeneratePullDraft::new(
                    serving.agent.as_ref().map(channel),
                    serving.config.clone(),
                    modes.clone(),
                    scope.clone(),
                ),
                generate_pending: GeneratePending::new(
                    comments.clone(),
                    serving.agent.as_ref().map(channel),
                    serving.config,
                    modes,
                    publisher,
                ),
                open_pull_request,
                comments,
                close_thread,
                write_pending,
                edit_pending,
                remove_pending,
                choose_mode,
                readiness,
                publish_review,
                sync_read_marks,
            }),
            git_dir,
            root,
        })
    }

    pub fn git_dir(&self) -> &PathBuf {
        &self.git_dir
    }

    /// The working tree the review is of.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }
}

pub fn config_cases() -> Result<ConfigUseCases> {
    Ok(ConfigUseCases::new(config_repository()?, lens_catalog()?))
}

/// The session this process was started from, read off what its harness
/// exported. Read here and not by the adapters, which take what they are given.
pub fn agent_here() -> Option<AgentAddress> {
    AgentAddress::from_env(|name| std::env::var(name).ok())
}

/// Which harness this process was started from, by the name the screen uses.
pub fn harness_here() -> Option<&'static str> {
    agent_here().map(|address| channel(&address).harness())
}

/// The one place that knows which harness speaks through which transport.
fn channel(address: &AgentAddress) -> Arc<dyn AgentChannel> {
    match address {
        AgentAddress::ClaudeCode { socket, token } => {
            Arc::new(ClaudeSocket::new(socket.clone(), token.clone()))
        }
        AgentAddress::Codex { thread } => Arc::new(CodexQueue::new("codex", thread.clone())),
    }
}

fn lens_catalog() -> Result<Arc<dyn LensCatalogRepository>> {
    Ok(Arc::new(JsonLensCatalog::new(
        crate::shared::paths::config_dir()?,
    )))
}

fn config_repository() -> Result<Arc<dyn ConfigRepository>> {
    Ok(Arc::new(JsonConfigRepository::new(
        crate::shared::paths::config_dir()?,
    )))
}

#[derive(Clone)]
pub struct ConfigUseCases {
    pub get: crate::config::application::GetConfig,
    pub set_setting: crate::config::application::SetSetting,
    pub reset_setting: crate::config::application::ResetSetting,
    pub list_lenses: ListLenses,
    pub add_lens: AddLens,
    pub update_lens: UpdateLens,
    pub enable_lens: EnableLens,
    pub disable_lens: DisableLens,
    pub delete_lens: DeleteLens,
}

impl ConfigUseCases {
    pub fn new(
        repo: Arc<dyn crate::config::domain::ConfigRepository>,
        lenses: Arc<dyn LensCatalogRepository>,
    ) -> Self {
        Self {
            list_lenses: ListLenses::new(lenses.clone()),
            add_lens: AddLens::new(lenses.clone()),
            update_lens: UpdateLens::new(lenses.clone()),
            enable_lens: EnableLens::new(lenses.clone()),
            disable_lens: DisableLens::new(lenses.clone()),
            delete_lens: DeleteLens::new(lenses),
            get: crate::config::application::GetConfig::new(repo.clone()),
            set_setting: crate::config::application::SetSetting::new(repo.clone()),
            reset_setting: crate::config::application::ResetSetting::new(repo),
        }
    }
}

pub fn check_head(root: &Path) -> Result<crate::diff::application::CheckHead> {
    let workspace = Workspace::discover(root)?;
    Ok(crate::diff::application::CheckHead::new(Arc::new(
        crate::diff::infra::GixHead::new(workspace.into_repo()),
    )))
}

pub fn server_factory(root: PathBuf) -> Result<Arc<dyn ServerUseCaseFactory>> {
    Ok(Arc::new(WorkspaceUseCases {
        root,
        config: config_repository()?,
        logins: Arc::default(),
    }))
}

/// A request gets one Git window, shared by all the use cases it invokes.
/// Keeping the factory instead of its result lets the next request follow refs.
pub trait ServerUseCaseFactory: Send + Sync {
    fn build(&self, request: ScopeRequest, agent: Option<AgentAddress>) -> Result<ServerUseCases>;
}

impl ServerUseCaseFactory for WorkspaceUseCases {
    fn build(&self, request: ScopeRequest, agent: Option<AgentAddress>) -> Result<ServerUseCases> {
        let serving = Serving {
            agent,
            config: self.config.clone(),
            logins: self.logins.clone(),
        };
        Ok(Ctx::assemble(&self.root, request, Some(serving))?
            .server
            .expect("a serving context always assembles the server's use cases"))
    }
}

struct WorkspaceUseCases {
    root: PathBuf,
    config: Arc<dyn ConfigRepository>,
    /// Here, because this outlives every request it builds for.
    logins: Arc<Logins>,
}
