//! The use cases a running server holds, and how it gets a fresh set per
//! request.
//!
//! Defined here, by the server that consumes them, so the server depends on
//! nothing in `cmd`. The composition root builds them and implements the
//! factory.

use crate::agent::domain::AgentAddress;
use crate::comments::application::{
    AddThread, ChooseMode, CloseThread, EditPending, GeneratePending, GeneratePullDraft,
    ListThreads, OpenPullRequest, PublishReview, PushBranch, RemovePending, ReplyToThread,
    ReviewReadiness, SendComment, ShowPullDraft, SyncReadMarks, WritePending,
};
use crate::diff::domain::ScopeRequest;
use crate::error::Result;
use crate::lenses::application::{
    AddLens, DeleteLens, DisableLens, EnableLens, GetReviewWithLenses, ListLenses, SeeLens,
    UpdateLens,
};
use crate::map::application::{GetFileDiff, GetFileLines, GetScope};
use crate::progress::application::{MarkViewed, UnmarkViewed};

/// What a running server holds, split the way its routes are: each route
/// asks for the group of its subject, and cannot reach past it.
#[derive(Clone)]
pub struct ServerUseCases {
    pub reading: ReadingCases,
    pub threads: ThreadCases,
    pub github: GitHubCases,
}

/// The review itself: what is in it, its diffs, and what has been read.
#[derive(Clone)]
pub struct ReadingCases {
    pub scope: GetScope,
    pub review: GetReviewWithLenses,
    pub see_lens: SeeLens,
    pub file_diff: GetFileDiff,
    pub file_lines: GetFileLines,
    pub mark_viewed: MarkViewed,
    pub unmark_viewed: UnmarkViewed,
}

/// The reviewer's conversation, and handing a thread to the agent.
#[derive(Clone)]
pub struct ThreadCases {
    pub add_thread: AddThread,
    pub list_threads: ListThreads,
    pub reply_to_thread: ReplyToThread,
    pub close_thread: CloseThread,
    pub send_comment: SendComment,
}

/// The pull request: its pending review, the mode, and opening one.
#[derive(Clone)]
pub struct GitHubCases {
    pub generate_pending: GeneratePending,
    pub write_pending: WritePending,
    pub edit_pending: EditPending,
    pub remove_pending: RemovePending,
    pub choose_mode: ChooseMode,
    pub readiness: ReviewReadiness,
    pub publish_review: PublishReview,
    pub sync_read_marks: SyncReadMarks,
    pub push_branch: PushBranch,
    pub show_pull_draft: ShowPullDraft,
    pub open_pull_request: OpenPullRequest,
    pub generate_pull_draft: GeneratePullDraft,
}

/// A request gets one Git window, shared by all the use cases it invokes.
/// Keeping the factory instead of its result lets the next request follow refs.
pub trait ServerUseCaseFactory: Send + Sync {
    fn build(&self, request: ScopeRequest, agent: Option<AgentAddress>) -> Result<ServerUseCases>;
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
