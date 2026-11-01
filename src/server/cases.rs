//! The use cases a running server holds, and how it gets a fresh set per
//! request.
//!
//! Defined here, by the server that consumes them, so the server depends on
//! nothing in `cmd`. The composition root builds them and implements the
//! factory.

use crate::agent::infra::AgentAddress;
use crate::comments::application::{
    ChooseMode, CloseThread, Comments, EditPending, GeneratePending, GeneratePullDraft,
    OpenPullRequest, PublishReview, PushBranch, RemovePending, ReviewReadiness, SendComment,
    SyncReadMarks, WritePending,
};
use crate::diff::infra::ScopeRequest;
use crate::error::Result;
use crate::lenses::application::{
    AddLens, DeleteLens, DisableLens, EnableLens, GetReviewWithLenses, ListLenses, SeeLens,
    UpdateLens,
};
use crate::map::application::{GetFileDiff, GetFileLines, GetScope};
use crate::progress::application::{MarkViewed, UnmarkViewed};

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
    pub push_branch: PushBranch,
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
