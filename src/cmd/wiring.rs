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
    AcknowledgeThread, AddThread, ChooseMode, CloseThread, CommentReach, Comments, EditPending,
    GeneratePending, GeneratePullDraft, ListThreads, OpenPullRequest, PublishReview,
    PullRequestGate, PushBranch, RemovePending, ReplyToThread, ReviewModes, ReviewReadiness,
    SendComment, SyncReadMarks, WritePending, WritePullDraft,
};
use crate::comments::domain::{
    BranchPusher, Clock, CommentStore, ModeStore, PendingReview, PullDraftStore, PullRequests,
    ReadFiles, ReadMarks,
};
use crate::comments::infra::{
    GhCredentials, GitHub, GitPush, JsonModeStore, JsonPullDraft, Logins, MarkdownComments,
    SystemClock, Unhosted,
};
use crate::config::domain::ConfigRepository;
use crate::config::infra::JsonConfigRepository;
use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
use crate::diff::domain::{CommitHistorySource, FileDiffSource, ReviewScopeSource};
use crate::diff::infra::{GixSource, ScopeRequest};
use crate::error::Result;
use crate::lenses::application::{
    AddLens, DeleteLens, DisableLens, EnableLens, GetReviewWithLenses, KeepAnswer, ListLenses,
    ListReadFiles, PendingAnswers, SeeLens, ShowAnswer, UpdateLens, WriteAnswer,
};
use crate::lenses::domain::{LensAnswerRepository, LensCatalogRepository};
use crate::lenses::infra::{JsonLensAnswers, JsonLensCatalog};
use crate::map::application::{
    AddBlock, AddFile, AddLineNote, CheckMap, DeriveMap, DiscardNote, ExportMap, FocusFile,
    GetFileDiff, GetFileLines, GetReview, GetScope, ImportMap, MapDerivation, MapEditor,
    MapReconciler, MapVersions, MoveBlock, RemoveBlock, RemoveFile, RemoveLineNote, ResetMap,
    RestoreNote, ShowMap, UpdateBlock, UpdateFile, UpdateLineNote,
};
use crate::map::domain::MapRepository;
use crate::map::infra::JsonMapRepository;
use crate::progress::application::{MarkViewed, ProgressStore, UnmarkViewed};
use crate::progress::domain::ProgressRepository;
use crate::progress::infra::JsonProgressRepository;
use crate::server::{ConfigUseCases, ServerUseCaseFactory, ServerUseCases};
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

    pub add_thread: AddThread,
    pub list_threads: ListThreads,
    pub reply_to_thread: ReplyToThread,
    pub acknowledge_thread: AcknowledgeThread,
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

    git_dir: PathBuf,
    root: PathBuf,
}

/// What a server adds to a context: the session it reports to, and where the
/// user's preferences are read from.
pub struct Serving {
    pub agent: Option<AgentAddress>,
    pub config: Arc<dyn ConfigRepository>,
}

impl Ctx {
    /// Choose the real implementations for the repository farol was invoked in.
    pub fn from_workspace(request: ScopeRequest) -> Result<Self> {
        Self::at(&std::env::current_dir()?, request)
    }

    pub fn at(root: &Path, request: ScopeRequest) -> Result<Self> {
        let workspace = Workspace::discover(root)?;
        let git_dir = workspace.git_dir().to_path_buf();
        let root = workspace.root();
        let name = workspace.name();
        let ports = Ports::of_workspace(workspace, &request, Arc::default())?;
        let shared = Shared::over(&ports);

        let reconciler = MapReconciler::new(shared.scope.clone(), shared.diffs.clone());
        let derivation = MapDerivation::new(
            shared.versions.clone(),
            shared.scope.clone(),
            reconciler,
            ports.maps.clone(),
        );
        let editor = MapEditor::new(
            derivation.clone(),
            shared.versions.clone(),
            ports.maps.clone(),
        );
        let show_answer = ShowAnswer::new(ports.lens_answers.clone());
        let write_answer = WriteAnswer::new(
            ports.lens_catalog.clone(),
            ports.lens_answers.clone(),
            shared.scope.clone(),
            shared.diffs.clone(),
            shared.versions.clone(),
        );
        let versions = shared.versions.clone();
        let scope = shared.scope.clone();

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
            import_map: ImportMap::new(scope.clone(), ports.maps.clone()),
            scope: GetScope::new(scope.clone()),

            acknowledge_thread: AcknowledgeThread::new(shared.comments.clone()),
            add_thread: shared.add_thread,
            list_threads: shared.list_threads,
            reply_to_thread: shared.reply_to_thread,
            close_thread: shared.close_thread,
            write_pending: shared.write_pending,
            edit_pending: shared.edit_pending,
            remove_pending: shared.remove_pending,
            choose_mode: shared.choose_mode,
            readiness: shared.readiness,
            publish_review: shared.publish_review,
            sync_read_marks: shared.sync_read_marks,
            write_pull_draft: WritePullDraft::new(ports.drafts.clone()),

            keep_answer: KeepAnswer::new(show_answer.clone(), write_answer.clone()),
            write_answer,
            pending_answers: PendingAnswers::new(
                ports.lens_catalog.clone(),
                ports.lens_answers.clone(),
                scope,
                versions,
            ),
            show_answer,

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

/// The ports one review runs on.
///
/// The real program chooses them in `of_workspace`; the tests hand in fakes.
/// Both then build the use cases through the same functions, so the server a
/// test exercises is wired exactly as the one a reviewer runs, rather than by a
/// second assembly that drifts from it.
pub struct Ports {
    pub scope: Arc<dyn ReviewScopeSource>,
    pub diffs: Arc<dyn FileDiffSource>,
    pub history: Arc<dyn CommitHistorySource>,
    pub comments: Arc<dyn CommentStore>,
    pub maps: Arc<dyn MapRepository>,
    pub progress: Arc<dyn ProgressRepository>,
    pub lens_catalog: Arc<dyn LensCatalogRepository>,
    pub lens_answers: Arc<dyn LensAnswerRepository>,
    pub modes: Arc<dyn ModeStore>,
    pub drafts: Arc<dyn PullDraftStore>,
    pub pulls: Arc<dyn PullRequests>,
    pub pending: Arc<dyn PendingReview>,
    pub marks: Arc<dyn ReadMarks>,
    pub pusher: Arc<dyn BranchPusher>,
    pub clock: Arc<dyn Clock>,
    /// The host the repository's remote is on, for the screen's login hint.
    pub host: Option<String>,
}

impl Ports {
    /// The real implementations for one repository.
    fn of_workspace(
        workspace: Workspace,
        request: &ScopeRequest,
        logins: Arc<Logins>,
    ) -> Result<Self> {
        let branch = workspace.branch().to_string();
        let remote = workspace.remote();
        let host = remote.as_ref().map(|remote| remote.host.clone());
        let comments = Arc::new(MarkdownComments::new(&workspace.store(), &workspace.root()));
        let maps = Arc::new(JsonMapRepository::new(workspace.store()));
        let progress = Arc::new(JsonProgressRepository::new(workspace.store()));
        let lens_answers = Arc::new(JsonLensAnswers::new(workspace.store().lenses_dir()));
        let modes = Arc::new(JsonModeStore::new(workspace.store()));
        let drafts = Arc::new(JsonPullDraft::new(workspace.store()));
        let pusher = Arc::new(GitPush::new(workspace.root()));
        // Read by the CLI too: the session writes answers from the terminal and
        // each one is checked against the user's lenses.
        let lens_catalog = lens_catalog()?;
        let source = Arc::new(GixSource::open(workspace.into_repo(), &branch, request)?);

        // One host behind three ports: each use case is handed the one it
        // needs, the way one GixSource stands behind the three diff ports.
        let (pulls, pending, marks): (
            Arc<dyn PullRequests>,
            Arc<dyn PendingReview>,
            Arc<dyn ReadMarks>,
        ) = match remote {
            Some(remote) => {
                let github = Arc::new(GitHub::new(
                    remote,
                    Arc::new(GhCredentials::new("gh")),
                    logins,
                ));
                (github.clone(), github.clone(), github)
            }
            None => {
                let unhosted = Arc::new(Unhosted);
                (unhosted.clone(), unhosted.clone(), unhosted)
            }
        };

        Ok(Self {
            scope: source.clone(),
            diffs: source.clone(),
            history: source,
            comments,
            maps,
            progress,
            lens_catalog,
            lens_answers,
            modes,
            drafts,
            pulls,
            pending,
            marks,
            pusher,
            clock: Arc::new(SystemClock),
            host,
        })
    }
}

/// The server's use cases over a set of ports, for one request.
///
/// Builds only what a server needs: a request used to assemble the whole CLI
/// bag as well and throw it away.
pub fn server_use_cases(ports: &Ports, serving: Serving) -> ServerUseCases {
    let shared = Shared::over(ports);
    let agent = serving.agent.as_ref().map(channel);
    ServerUseCases {
        scope: GetScope::new(shared.scope.clone()),
        see_lens: SeeLens::new(ports.lens_answers.clone(), shared.progress.clone()),
        file_diff: GetFileDiff::new(shared.diffs.clone()),
        file_lines: GetFileLines::new(shared.scope.clone()),
        mark_viewed: MarkViewed::new(shared.progress.clone()),
        unmark_viewed: UnmarkViewed::new(shared.progress.clone()),
        send_comment: SendComment::new(
            shared.comments.clone(),
            agent.clone(),
            serving.config.clone(),
            shared.modes.clone(),
        ),
        generate_pull_draft: GeneratePullDraft::new(
            agent.clone(),
            serving.config.clone(),
            shared.modes.clone(),
            shared.scope.clone(),
        ),
        generate_pending: GeneratePending::new(
            shared.comments.clone(),
            agent,
            serving.config,
            shared.modes.clone(),
            ports.pending.clone(),
        ),
        push_branch: PushBranch::new(shared.gate.clone(), ports.pusher.clone()),
        open_pull_request: OpenPullRequest::new(
            shared.gate.clone(),
            ports.pulls.clone(),
            ports.pusher.clone(),
            shared.scope.clone(),
            ports.drafts.clone(),
        ),
        review: shared.review,
        add_thread: shared.add_thread,
        list_threads: shared.list_threads,
        reply_to_thread: shared.reply_to_thread,
        close_thread: shared.close_thread,
        write_pending: shared.write_pending,
        edit_pending: shared.edit_pending,
        remove_pending: shared.remove_pending,
        choose_mode: shared.choose_mode,
        readiness: shared.readiness,
        publish_review: shared.publish_review,
        sync_read_marks: shared.sync_read_marks,
    }
}

/// The files read, as the lenses' use case answers it, behind the port the
/// comments' use cases take: they learn a list of paths, not how it is made.
struct LensAwareReads(ListReadFiles);

impl ReadFiles for LensAwareReads {
    fn read(&self) -> Result<Vec<String>> {
        self.0.execute()
    }
}

/// What the CLI and the server both build from the ports, built once here so
/// the two can only differ in what they add on top.
struct Shared {
    scope: ReviewScope,
    diffs: FileDiffs,
    versions: MapVersions,
    progress: ProgressStore,
    modes: ReviewModes,
    gate: PullRequestGate,
    review: GetReviewWithLenses,
    comments: Comments,
    add_thread: AddThread,
    reply_to_thread: ReplyToThread,
    list_threads: ListThreads,
    close_thread: CloseThread,
    write_pending: WritePending,
    edit_pending: EditPending,
    remove_pending: RemovePending,
    choose_mode: ChooseMode,
    readiness: ReviewReadiness,
    publish_review: PublishReview,
    sync_read_marks: SyncReadMarks,
}

impl Shared {
    fn over(ports: &Ports) -> Self {
        let scope = ReviewScope::new(ports.scope.clone());
        let diffs = FileDiffs::new(ports.diffs.clone());
        let history = CommitHistory::new(ports.history.clone());
        let comments = Comments::new(
            ports.comments.clone(),
            GetScope::new(scope.clone()),
            CommentReach::new(scope.clone(), diffs.clone()),
            ports.clock.clone(),
        );
        let versions = MapVersions::new(scope.clone(), history.clone(), ports.maps.clone());
        let progress = ProgressStore::new(ports.progress.clone(), diffs.clone());
        let modes = ReviewModes::new(ports.modes.clone());
        let review = GetReviewWithLenses::new(
            GetReview::new(versions.clone(), scope.clone(), progress.clone()),
            ports.lens_catalog.clone(),
            ports.lens_answers.clone(),
        );
        let read: Arc<dyn ReadFiles> = Arc::new(LensAwareReads(ListReadFiles::new(
            progress.clone(),
            scope.clone(),
            review.clone(),
        )));
        let gate = PullRequestGate::new(ports.pulls.clone(), scope.clone(), history);
        Self {
            readiness: ReviewReadiness::new(
                gate.clone(),
                modes.clone(),
                comments.clone(),
                ports.pending.clone(),
                ports.host.clone(),
                read.clone(),
                ports.drafts.clone(),
            ),
            sync_read_marks: SyncReadMarks::new(gate.clone(), ports.marks.clone(), read.clone()),
            publish_review: PublishReview::new(
                comments.clone(),
                gate.clone(),
                ports.pending.clone(),
                ports.marks.clone(),
                read,
            ),
            close_thread: CloseThread::new(comments.clone()),
            add_thread: AddThread::new(comments.clone()),
            reply_to_thread: ReplyToThread::new(comments.clone()),
            list_threads: ListThreads::new(comments.clone()),
            write_pending: WritePending::new(comments.clone(), gate.clone(), ports.pending.clone()),
            edit_pending: EditPending::new(ports.pending.clone()),
            remove_pending: RemovePending::new(comments.clone(), ports.pending.clone()),
            choose_mode: ChooseMode::new(modes.clone()),
            scope,
            diffs,
            versions,
            progress,
            modes,
            gate,
            review,
            comments,
        }
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

impl ServerUseCaseFactory for WorkspaceUseCases {
    fn build(&self, request: ScopeRequest, agent: Option<AgentAddress>) -> Result<ServerUseCases> {
        let workspace = Workspace::discover(&self.root)?;
        let ports = Ports::of_workspace(workspace, &request, self.logins.clone())?;
        Ok(server_use_cases(
            &ports,
            Serving {
                agent,
                config: self.config.clone(),
            },
        ))
    }
}

struct WorkspaceUseCases {
    root: PathBuf,
    config: Arc<dyn ConfigRepository>,
    /// Here, because this outlives every request it builds for.
    logins: Arc<Logins>,
}
