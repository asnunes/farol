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
use axum::routing::{delete, get, post, put};
use axum::{Extension, Json, Router};
use serde::Deserialize;

use tokio::sync::broadcast;

use super::{AppState, SessionConfig, assets, view};
use crate::cmd::ServerUseCases;
use crate::comments::domain::Verdict;
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
        .route("/api/publish", get(readiness).post(publish))
        .route("/api/publish/ticks", post(ticks))
        .route("/api/token", put(save_token))
        .route_layer(middleware::from_fn_with_state(state.clone(), resolve))
        .route("/api/session", put(configure))
        .route("/api/watch", get(watch))
        .route("/health", get(health))
        .fallback(assets::handler)
        .with_state(state)
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
        Ok(found) => Json(view::CommentsView::of(&found)).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct NewComment {
    path: String,
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
        .add(&body.path, body.from, body.to, &body.body)
    {
        Ok(comment) => Json(view::CommentView::of(&comment)).into_response(),
        Err(e) => fail(e),
    }
}

async fn close_comment(
    Extension(cases): Extension<ServerUseCases>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match cases.comments.close(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

/// Whether the review can be sent, and what is in the way when it cannot.
///
/// The page asks this on load and keeps asking while the answer is no, because
/// the answer changes elsewhere: a token pasted into a file, a branch pushed
/// from a terminal, a pull request opened in a browser.
async fn readiness(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    let readiness = cases.readiness;
    match away(move || readiness.execute()).await {
        Ok(standing) => Json(view::ReadinessView::of(
            &standing.readiness,
            &standing.branch,
            standing.host.as_deref(),
        ))
        .into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct ReviewToSend {
    verdict: String,
    summary: String,
}

/// Send it. The one irreversible thing farol does, which is why every reason it
/// should not happen is decided in the use case rather than here.
async fn publish(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<ReviewToSend>,
) -> impl IntoResponse {
    let Some(verdict) = verdict(&body.verdict) else {
        return fail(Error::msg(format!("unknown verdict '{}'", body.verdict)));
    };
    let publish = cases.publish_review;
    match away(move || publish.execute(verdict, &body.summary)).await {
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

/// The ticks on their own, for the review that cannot be sent or has nothing
/// to say. Same call the publish route makes last, without the review in front
/// of it.
async fn ticks(Extension(cases): Extension<ServerUseCases>) -> impl IntoResponse {
    let publish = cases.publish_review;
    match away(move || publish.ticks_only()).await {
        Ok(read) => Json(serde_json::json!({ "read": read })).into_response(),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
struct NewToken {
    host: String,
    token: String,
}

/// Take the token and say nothing back.
///
/// No route ever answers with it and nothing logs it: a credential that can be
/// read back out of the thing holding it is a credential with two homes.
async fn save_token(
    Extension(cases): Extension<ServerUseCases>,
    Json(body): Json<NewToken>,
) -> impl IntoResponse {
    match cases.save_token.execute(&body.host, &body.token) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => fail(e),
    }
}

fn verdict(name: &str) -> Option<Verdict> {
    match name {
        "comment" => Some(Verdict::Comment),
        "requestChanges" => Some(Verdict::RequestChanges),
        "approve" => Some(Verdict::Approve),
        _ => None,
    }
}

/// Run something that talks to the network off the runtime's own threads.
///
/// Publishing is blocking HTTP inside an async server. Left where it is, one
/// slow call to GitHub holds a worker thread that every other request on this
/// port is queued behind.
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
    use crate::comments::application::Comments;
    use crate::comments::application::{PublishReview, ReviewReadiness, SaveToken};
    use crate::comments::domain::Readiness;
    use crate::progress::application::ProgressStore;
    use crate::server::{Registry, ServerEntry};
    use crate::testing::{
        FakeCredentials, FakeDiffSource, FakePublisher, InMemoryComments,
        InMemoryProgressRepository,
    };
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
        let scope = GetScope::new(ReviewScope::new(source.clone()));
        let comments = Comments::new(
            Arc::new(MarkdownComments::new(&store, dir.path())),
            scope,
            FileDiffs::new(source),
        );
        (dir, comments)
    }

    /// The three publishing use cases over a publisher that answers whatever
    /// the test needs and keeps what it was handed instead of sending it.
    fn publishing(
        paths: &[&str],
        readiness: Readiness,
    ) -> (
        ReviewReadiness,
        Arc<PublishReview>,
        Arc<SaveToken>,
        Arc<FakePublisher>,
    ) {
        use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};

        let source = Arc::new(FakeDiffSource::with_paths(paths));
        let publisher = Arc::new(FakePublisher::blocked(readiness));
        (
            ReviewReadiness::new(
                publisher.clone(),
                ReviewScope::new(source.clone()),
                Some("github.com".into()),
            ),
            Arc::new(PublishReview::new(
                Arc::new(InMemoryComments::default()),
                publisher.clone(),
                ReviewScope::new(source.clone()),
                FileDiffs::new(source.clone()),
                CommitHistory::new(source.clone()),
                ProgressStore::new(
                    Arc::new(crate::testing::InMemoryProgressRepository::default()),
                    FileDiffs::new(source),
                ),
            )),
            Arc::new(SaveToken::new(
                Arc::new(FakeCredentials::default()),
                Some("github.com".into()),
            )),
            publisher,
        )
    }

    pub(in crate::server) fn use_cases() -> (tempfile::TempDir, ServerUseCases) {
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::{GetFileDiff, GetFileLines, GetReview};
        use crate::progress::application::{MarkViewed, UnmarkViewed};

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
        let (readiness, publish_review, save_token, _) = publishing(
            &paths,
            Readiness::Ready {
                pull_request: 12,
                id: "PR_kwDO".into(),
                mine: false,
                head: "head".into(),
            },
        );
        (
            dir,
            ServerUseCases {
                scope: crate::map::application::GetScope::new(scope.clone()),
                review: GetReview::new(maps.versions, scope, progress.clone()),
                file_diff: GetFileDiff::new(diffs),
                file_lines: GetFileLines::new(ReviewScope::new(Arc::new(
                    FakeDiffSource::with_paths(&paths),
                ))),
                mark_viewed: MarkViewed::new(progress.clone()),
                unmark_viewed: UnmarkViewed::new(progress),
                comments,
                readiness,
                publish_review,
                save_token,
            },
        )
    }

    #[tokio::test]
    async fn a_review_that_cannot_be_assembled_is_a_bad_request_not_a_silent_empty_page() {
        // If the progress store is unreadable the screen must say so rather
        // than render as though nothing had been read.
        use crate::diff::application::{FileDiffs, ReviewScope};
        use crate::map::application::{GetFileDiff, GetFileLines, GetReview};
        use crate::progress::application::{MarkViewed, ProgressStore, UnmarkViewed};

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
        let (readiness, publish_review, save_token, _) = publishing(&paths, Readiness::NoToken);
        let use_cases = ServerUseCases {
            scope: crate::map::application::GetScope::new(ReviewScope::new(Arc::new(
                FakeDiffSource::with_paths(&paths),
            ))),
            review: GetReview::new(
                maps.versions,
                ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&paths))),
                broken.clone(),
            ),
            file_diff: GetFileDiff::new(diffs),
            file_lines: GetFileLines::new(ReviewScope::new(Arc::new(FakeDiffSource::with_paths(
                &paths,
            )))),
            mark_viewed: MarkViewed::new(broken.clone()),
            unmark_viewed: UnmarkViewed::new(broken),
            comments,
            readiness,
            publish_review,
            save_token,
        };
        let (state, _) = AppState::new(
            session(use_cases, _dir.path()),
            tokio::sync::watch::channel(false).1,
        );

        let response = router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/review")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn the_state_of_publishing_is_answered_as_a_reason_and_not_as_a_flag() {
        // Each of these sends the reader somewhere different. A boolean would
        // send them to the token they already have.
        for (readiness, state) in [
            (Readiness::NoToken, "noToken"),
            (Readiness::TokenRefused, "tokenRefused"),
            (Readiness::BranchNotPushed, "branchNotPushed"),
            (
                Readiness::NoPullRequest {
                    open_at: "https://example.test/compare".into(),
                },
                "noPullRequest",
            ),
        ] {
            let body = ask(readiness).await;

            assert_eq!(body["state"], state);
            assert_eq!(
                body["branch"], "feature/x",
                "the panel spells commands with it"
            );
        }
    }

    #[tokio::test]
    async fn a_pull_request_that_is_there_comes_back_with_its_number() {
        let body = ask(Readiness::Ready {
            pull_request: 12,
            id: "PR_kwDO".into(),
            mine: false,
            head: "head".into(),
        })
        .await;

        assert_eq!(body["state"], "ready");
        assert_eq!(body["pullRequest"], 12);
    }

    #[tokio::test]
    async fn nowhere_to_open_a_pull_request_means_no_link_to_offer() {
        // The link is the whole of what that state offers, and a state that has
        // none must not hand the page an empty string to render as a button.
        let body = ask(Readiness::BranchNotPushed).await;

        assert!(body.get("openAt").is_none(), "{body}");
        assert!(body.get("pullRequest").is_none(), "{body}");
    }

    #[tokio::test]
    async fn a_verdict_the_api_does_not_have_is_refused_before_anything_is_sent() {
        let (_dir, use_cases) = use_cases();
        let (state, _) = AppState::new(
            session(use_cases, _dir.path()),
            tokio::sync::watch::channel(false).1,
        );

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/publish")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"verdict":"lgtm","summary":"Reads well."}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn the_token_route_answers_with_nothing_at_all() {
        // Not even an echo. A route that hands the token back is a second place
        // it can be read from.
        let (_dir, use_cases) = use_cases();
        let (state, _) = AppState::new(
            session(use_cases, _dir.path()),
            tokio::sync::watch::channel(false).1,
        );

        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/token")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"host":"github.com","token":"ghp_abc123"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(read(response).await.is_empty());
    }

    /// Ask `/api/publish` over a publisher parked in one state.
    async fn ask(readiness: Readiness) -> serde_json::Value {
        let (_dir, mut use_cases) = use_cases();
        let (asked, publish_review, save_token, _) = publishing(&["a.rs"], readiness);
        use_cases.readiness = asked;
        use_cases.publish_review = publish_review;
        use_cases.save_token = save_token;
        let (state, _) = AppState::new(
            session(use_cases, _dir.path()),
            tokio::sync::watch::channel(false).1,
        );

        let response = router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/publish")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        serde_json::from_str(&read(response).await).unwrap()
    }

    async fn read(response: axum::response::Response) -> String {
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        String::from_utf8(body.to_vec()).unwrap()
    }
}
