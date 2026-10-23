//! What the browser asks for.
//!
//! Handlers only: each one unpacks the request, calls one use case, and turns
//! the answer into a response. No business logic passes through here — that is
//! the point of the layer.

use std::sync::Arc;

use axum::extract::Request;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use axum::routing::{any, delete, get, post, put};
use axum::{Extension, Json, Router};
use serde::Deserialize;

use tokio::sync::broadcast;

use super::{AppState, SessionConfig, assets, view};
use crate::cmd::ServerUseCases;
use crate::config::presentation::ConfigView;
use crate::diff::domain::Side;
use crate::error::Error;

/// The whole HTTP surface. Everything it serves arrives injected, so the routes
/// can be exercised over fakes with nothing listening on a port.
pub(super) fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/review", get(review))
        .route("/api/file", get(file))
        .route("/api/lines", get(lines))
        .route("/api/viewed", post(viewed))
        .route("/api/comments", get(comments).post(write_comment))
        .route("/api/comments/{id}", delete(close_comment))
        .route("/api/comments/{id}/send", post(send_comment))
        .route("/api/comments/{id}/replies", post(reply_comment))
        .route("/api/comments/{id}/generate", post(generate_pending))
        .route(
            "/api/comments/{id}/pending",
            post(write_pending).put(edit_pending).delete(remove_pending),
        )
        .route("/api/github", get(pull_request))
        .route("/api/github/review", post(submit_review))
        .route("/api/mode", put(choose_mode))
        .route_layer(middleware::from_fn_with_state(state.clone(), resolve))
        .route("/api/config", get(get_config).put(set_config))
        .route("/api/config/{key}", delete(reset_config))
        .route("/api/session", put(configure))
        .route("/api/watch", get(watch))
        .route("/health", get(health))
        .route("/api/{*path}", any(|| async { StatusCode::NOT_FOUND }))
        .fallback(assets::handler)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            super::local_request::local_request,
        ))
        .with_state(state)
}

async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.preferences.get.execute() {
        Ok(config) => Json(ConfigView::from(config)).into_response(),
        Err(error) => fail(error),
    }
}

async fn set_config(
    State(state): State<Arc<AppState>>,
    Json(setting): Json<SettingBody>,
) -> impl IntoResponse {
    let key = match crate::config::domain::SettingKey::parse(&setting.key) {
        Ok(key) => key,
        Err(error) => return fail(error.into()),
    };
    match state.preferences.set_setting.execute(key, setting.value) {
        Ok(config) => Json(ConfigView::from(config)).into_response(),
        Err(error) => fail(error),
    }
}

/// Back to the default, answering with the configuration as it now stands so
/// the page redraws the field from the server rather than guessing.
async fn reset_config(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let key = match crate::config::domain::SettingKey::parse(&key) {
        Ok(key) => key,
        Err(error) => return fail(error.into()),
    };
    match state.preferences.reset_setting.execute(key) {
        Ok(config) => Json(ConfigView::from(config)).into_response(),
        Err(error) => fail(error),
    }
}

/// The value as JSON wrote it. Whether it suits the key is the setting's to
/// decide, so a mismatch is refused there and names what the key takes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingBody {
    key: String,
    value: crate::config::domain::SettingValue,
}

/// Who is answering on this port.
///
/// It carries the whole entry rather than an empty 200 because a port that was
/// let go and taken by another server answers just as readily — what settles it
/// is the process id coming back the same.
async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.session.identity())
}

async fn resolve(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> axum::response::Response {
    match away(move || state.session.open()).await {
        Ok(cases) => {
            request.extensions_mut().insert(cases);
            next.run(request).await
        }
        Err(e) => fail(e),
    }
}

async fn configure(
    State(state): State<Arc<AppState>>,
    Json(config): Json<SessionConfig>,
) -> impl IntoResponse {
    let session = state.session.clone();
    match away(move || session.configure(config)).await {
        Ok(identity) => {
            let _ = state.changes.send("map".to_string());
            // The session may have changed, and with it the name on the Send
            // button, which the page reads off the comments.
            let _ = state.changes.send("comments".to_string());
            Json(identity).into_response()
        }
        Err(e) => fail(e),
    }
}

/// Everything the screen needs to draw itself, in one call.
async fn review(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    match cases.review.execute() {
        Ok(snapshot) => Json(view::ReviewView::build(&snapshot)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct PathQuery {
    path: String,
}

async fn file(
    Extension(cases): Extension<ServerUseCases>,
    Query(q): Query<PathQuery>,
) -> impl IntoResponse {
    match cases.file_diff.execute(&q.path) {
        Ok(diff) => Json(view::FileDiffView::of(&diff)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct RangeQuery {
    path: String,
    from: u32,
    to: u32,
}

/// A stretch of the file the diff never printed, for the reader opening a gap.
///
/// Text and nothing else: which line each string is, and which line it was
/// before the change, the page works out from the hunks it already has.
async fn lines(
    Extension(cases): Extension<ServerUseCases>,
    Query(q): Query<RangeQuery>,
) -> impl IntoResponse {
    match cases.file_lines.execute(&q.path, q.from, q.to) {
        Ok(lines) => Json(view::LinesView { lines }).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct ViewedBody {
    path: String,
    viewed: bool,
}

async fn viewed(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<ViewedBody>,
) -> impl IntoResponse {
    let result = if body.viewed {
        cases.mark_viewed.execute(&body.path, &now())
    } else {
        cases.unmark_viewed.execute(&body.path)
    };
    match result {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

/// What the reviewer wrote back, which is everything still waiting for an
/// answer: closing a comment removes it, so there is nothing here to filter.
async fn comments(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    match cases.comments.all() {
        Ok(found) => {
            Json(view::CommentsView::of(&found, cases.send_comment.harness())).into_response()
        }
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct NewComment {
    path: String,
    /// Which side the numbers are counted on. Absent is the new side, which is
    /// what every comment written before there was a choice was about.
    #[serde(default)]
    side: Side,
    from: u32,
    to: u32,
    body: String,
}

async fn write_comment(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<NewComment>,
) -> impl IntoResponse {
    match cases
        .comments
        .add(&body.path, body.side, body.from, body.to, &body.body)
    {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloseQuery {
    /// Set once the reviewer confirmed that the thread's pending comment goes
    /// too. Without it a thread with one is refused, as in the terminal.
    #[serde(default)]
    discard_pending: bool,
}

async fn close_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Query(q): Query<CloseQuery>,
) -> impl IntoResponse {
    match away(move || cases.close_thread.execute(&id, q.discard_pending)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

/// The pull request, its mode and each thread's pending comment. Asked of
/// GitHub, so off the request thread.
async fn pull_request(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    match away(move || cases.readiness.execute()).await {
        Ok(standing) => Json(view::PullRequestView::of(&standing)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct PendingBody {
    body: String,
}

/// The reviewer's own words for the pull request: written in the box, or
/// taken from a turn of the conversation. Either way it is the reviewer who
/// sends them, so they are signed by the reviewer.
async fn write_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Json(pending): Json<PendingBody>,
) -> impl IntoResponse {
    let by = crate::comments::domain::Author::Reviewer;
    match away(move || cases.write_pending.execute(&id, &pending.body, by)).await {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

async fn edit_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Json(pending): Json<PendingBody>,
) -> impl IntoResponse {
    match away(move || cases.edit_pending.execute(&id, &pending.body)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

async fn remove_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match away(move || cases.remove_pending.execute(&id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize, Default)]
struct GenerateBody {
    /// The reviewer's note for this one comment, from "Generate with
    /// instructions…". Their own words, going to their own agent, the same as
    /// a reply is.
    #[serde(default)]
    instruction: String,
}

async fn generate_pending(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    body: Option<Json<GenerateBody>>,
) -> impl IntoResponse {
    let instruction = body.map(|Json(b)| b.instruction).unwrap_or_default();
    match away(move || cases.generate_pending.execute(&id, &instruction)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum VerdictBody {
    Comment,
    RequestChanges,
    Approve,
}

#[derive(Deserialize)]
struct SubmitBody {
    verdict: VerdictBody,
    #[serde(default)]
    summary: String,
}

/// Only the verdict and the summary travel: what is submitted is the pending
/// review already on the pull request, never text the request carries.
async fn submit_review(
    Extension(cases): Extension<ServerUseCases>,
    Json(submit): Json<SubmitBody>,
) -> impl IntoResponse {
    use crate::comments::domain::Verdict;
    let verdict = match submit.verdict {
        VerdictBody::Comment => Verdict::Comment,
        VerdictBody::RequestChanges => Verdict::RequestChanges,
        VerdictBody::Approve => Verdict::Approve,
    };
    match away(move || cases.publish_review.execute(verdict, &submit.summary)).await {
        Ok(sent) => Json(view::SentView {
            url: sent.url,
            comments: sent.comments,
            read: sent.read,
            read_failed: sent.read_failed,
        })
        .into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct ModeBody {
    mode: crate::comments::domain::ReviewMode,
}

async fn choose_mode(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<ModeBody>,
) -> impl IntoResponse {
    match away(move || cases.choose_mode.execute(body.mode)).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct NewReply {
    body: String,
}

/// The reviewer writing back under a comment. Sending it on is a second call,
/// the same one the Send button makes.
async fn reply_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
    Json(reply): Json<NewReply>,
) -> impl IntoResponse {
    let author = crate::comments::domain::Author::Reviewer;
    match away(move || cases.comments.reply(&id, author, &reply.body)).await {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

/// Hand one comment to the agent session. Only the id travels: the text is
/// built on the server from the comment as stored.
async fn send_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match away(move || cases.send_comment.execute(&id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

/// Run synchronous repository work off the async request thread.
async fn away<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(e) => Err(Error::msg(format!("the request could not be run: {e}"))),
    }
}

/// One-way channel: the page finds out that the repository moved without
/// polling for it.
///
/// The kind travels twice, as the event name and as the data. The data is not
/// redundant: a message whose data buffer is empty is dropped by the browser
/// rather than dispatched, so an event sent without one reaches curl and never
/// reaches a page.
async fn watch(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut rx = state.changes.subscribe();
    let mut stopping = state.stopping.clone();
    let stream = async_stream::stream! {
        loop {
            tokio::select! {
                // Whichever comes first, and the second arm is why the process
                // can be stopped at all: this stream is what graceful shutdown
                // would otherwise wait on forever.
                nudge = rx.recv() => match nudge {
                    Ok(kind) => yield Ok::<_, std::convert::Infallible>(Event::default().event(&kind).data(&kind)),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                },
                _ = asked_to_stop(&mut stopping) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

/// Resolves once the process has been asked to stop.
///
/// `wait_for` rather than `changed`, so a connection that arrives after the
/// signal reads the state instead of waiting for a second one that is never
/// coming. It is a function of its own because the borrow the watch hands back
/// is not `Send`, and the stream it is selected in has to be.
async fn asked_to_stop(stopping: &mut tokio::sync::watch::Receiver<bool>) {
    let _ = stopping.wait_for(|asked| *asked).await;
}

/// Every failure here is the caller asking for something that is not under
/// review, which is a bad request rather than a broken server.
fn fail(e: Error) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, e.to_string()).into_response()
}

fn now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

#[cfg(test)]
pub(super) mod tests {
    //! Only what a request over the wire cannot reach.
    //!
    //! The routes themselves are exercised in `tests/server.rs`, against a real
    //! process: the browser talks to one, and the composition root and the
    //! store are part of what can break. What is left here needs a collaborator
    //! that fails, which no amount of driving the real binary can arrange.

    use super::*;
    use crate::cmd::ServerUseCases;
    use crate::comments::application::{CommentReach, Comments};
    use crate::progress::application::ProgressStore;
    use crate::server::{Registry, ServerEntry};
    use crate::testing::{FakeDiffSource, InMemoryProgressRepository};
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::Arc;
    use tower::ServiceExt as _;

    /// Who the server would say it is. Nothing under test here asks, but the
    /// router carries it.
    fn identity() -> ServerEntry {
        ServerEntry {
            port: 4600,
            pid: std::process::id(),
            repo: std::path::PathBuf::from("/repo"),
            branch: "feature/x".into(),
            base: "main".into(),
        }
    }

    pub(in crate::server) fn session(
        cases: ServerUseCases,
        root: &std::path::Path,
    ) -> super::super::session::Session {
        super::super::session::Session::new(
            Arc::new(crate::testing::FixedServerUseCases(cases)),
            SessionConfig::default(),
            identity(),
            Arc::new(Registry::at(root).unwrap()),
        )
    }

    /// The use cases over fakes, shared with the tests that start a real
    /// listener next door.
    /// Comments over a real store in a folder that goes away with the test.
    ///
    /// The real one rather than a stand-in: a folder of markdown files is
    /// microseconds to write and it puts the header parser in the path, which
    /// is the fragile part. The directory comes back with it because it has to
    /// outlive what is built on top of it.
    fn comments(paths: &[&str]) -> (tempfile::TempDir, Comments) {
        use crate::comments::infra::MarkdownComments;
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::GetScope;

        let dir = tempfile::tempdir().unwrap();
        let store = crate::shared::paths::Store::new(dir.path(), "feature/x");
        let source = Arc::new(FakeDiffSource::with_paths(paths));
        let scope = ReviewScope::new(source.clone());
        let comments = Comments::new(
            Arc::new(MarkdownComments::new(&store, dir.path())),
            GetScope::new(scope.clone()),
            CommentReach::new(scope, FileDiffs::new(source)),
        );
        (dir, comments)
    }

    pub(in crate::server) fn use_cases() -> (tempfile::TempDir, ServerUseCases) {
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::GetReview;

        let paths = ["a.rs", "b.rs", "Cargo.lock"];
        let diffs = FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        let scope = ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        let maps = crate::testing::services(
            FakeDiffSource::with_paths(&paths),
            Arc::new(crate::testing::InMemoryMapRepository::new()),
        );
        let progress = ProgressStore::new(
            Arc::new(InMemoryProgressRepository::default()),
            diffs.clone(),
        );
        let (dir, comments) = comments(&paths);
        (
            dir,
            assemble(
                &paths,
                comments,
                GetReview::new(maps.versions, scope, progress.clone()),
                progress,
            ),
        )
    }

    /// The use cases a server holds, over the fakes, with the two that differ
    /// between tests handed in.
    fn assemble(
        paths: &[&str],
        comments: Comments,
        review: crate::map::application::GetReview,
        progress: ProgressStore,
    ) -> ServerUseCases {
        use crate::comments::application::{
            ChooseMode, CloseThread, EditPending, GeneratePending, PublishReview, PullRequestGate,
            RemovePending, ReviewModes, ReviewReadiness, SendComment, WritePending,
        };
        use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
        use crate::map::application::{GetFileDiff, GetFileLines};
        use crate::progress::application::{MarkViewed, UnmarkViewed};

        let source = Arc::new(FakeDiffSource::with_paths(paths));
        let scope = ReviewScope::new(source.clone());
        let publisher: Arc<dyn crate::comments::domain::ReviewPublisher> =
            Arc::new(crate::comments::infra::Unhosted);
        let config = Arc::new(crate::testing::InMemoryConfigRepository::default());
        let modes = ReviewModes::new(Arc::new(crate::testing::InMemoryModeStore::default()));
        let gate = PullRequestGate::new(
            publisher.clone(),
            scope.clone(),
            CommitHistory::new(source.clone()),
        );
        ServerUseCases {
            scope: crate::map::application::GetScope::new(scope.clone()),
            review,
            file_diff: GetFileDiff::new(FileDiffs::new(source.clone())),
            file_lines: GetFileLines::new(scope.clone()),
            mark_viewed: MarkViewed::new(progress.clone()),
            unmark_viewed: UnmarkViewed::new(progress.clone()),
            send_comment: SendComment::new(comments.clone(), None, config.clone(), modes.clone()),
            generate_pending: GeneratePending::new(
                comments.clone(),
                None,
                config,
                modes.clone(),
                publisher.clone(),
            ),
            close_thread: CloseThread::new(comments.clone(), publisher.clone()),
            write_pending: WritePending::new(comments.clone(), gate.clone(), publisher.clone()),
            edit_pending: EditPending::new(comments.clone(), publisher.clone()),
            remove_pending: RemovePending::new(comments.clone(), publisher.clone()),
            choose_mode: ChooseMode::new(modes.clone()),
            readiness: ReviewReadiness::new(
                gate.clone(),
                modes,
                comments.clone(),
                publisher.clone(),
                scope.clone(),
                None,
                progress.clone(),
            ),
            publish_review: PublishReview::new(comments.clone(), gate, publisher, scope, progress),
            comments,
        }
    }

    #[tokio::test]
    async fn a_review_that_cannot_be_assembled_is_a_bad_request_not_a_silent_empty_page() {
        // If the progress store is unreadable the screen must say so rather
        // than render as though nothing had been read.
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::GetReview;
        use crate::progress::application::ProgressStore;

        let paths = ["a.rs"];
        let diffs = FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&paths)));
        let maps = crate::testing::services(
            FakeDiffSource::with_paths(&paths),
            Arc::new(crate::testing::InMemoryMapRepository::new()),
        );
        // A map has to exist, or the request fails looking for one and never
        // reaches the collaborator this test is about.
        maps.editor
            .edit(|_| Ok::<_, crate::map::domain::MapError>(()))
            .unwrap();
        let broken = ProgressStore::new(
            Arc::new(crate::testing::BrokenProgressRepository),
            diffs.clone(),
        );
        let (_dir, comments) = comments(&paths);
        let use_cases = assemble(
            &paths,
            comments,
            GetReview::new(
                maps.versions,
                ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths))),
                broken.clone(),
            ),
            broken,
        );
        let (state, _) = AppState::new(
            session(use_cases, _dir.path()),
            crate::cmd::ConfigUseCases::new(Arc::new(
                crate::testing::InMemoryConfigRepository::default(),
            )),
            tokio::sync::watch::channel(false).1,
        );

        let response = router(state)
            .oneshot(
                Request::builder()
                    .header("host", "127.0.0.1:4600")
                    .uri("/api/review")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
