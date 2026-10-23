//! What the browser asks for: every path, in one place, and the handlers in a
//! file per subject beside it.
//!
//! Handlers only: each one unpacks the request, calls one use case, and turns
//! the answer into a response. No business logic passes through here — that is
//! the point of the layer.

use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{any, delete, get, post, put};

mod config;
mod github;
mod review;
mod session;
mod threads;
mod watch;

use super::{AppState, assets, view};
use crate::error::Error;

/// The whole HTTP surface. Everything it serves arrives injected, so the routes
/// can be exercised over fakes with nothing listening on a port.
pub(super) fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/review", get(review::review))
        .route("/api/file", get(review::file))
        .route("/api/lines", get(review::lines))
        .route("/api/viewed", post(review::viewed))
        .route(
            "/api/comments",
            get(threads::comments).post(threads::write_comment),
        )
        .route("/api/comments/{id}", delete(threads::close_comment))
        .route("/api/comments/{id}/send", post(threads::send_comment))
        .route("/api/comments/{id}/replies", post(threads::reply_comment))
        .route(
            "/api/comments/{id}/generate",
            post(github::generate_pending),
        )
        .route(
            "/api/comments/{id}/pending",
            post(github::write_pending)
                .put(github::edit_pending)
                .delete(github::remove_pending),
        )
        .route("/api/github", get(github::pull_request))
        .route("/api/github/review", post(github::submit_review))
        .route("/api/mode", put(github::choose_mode))
        .route_layer(middleware::from_fn_with_state(state.clone(), resolve))
        .route(
            "/api/config",
            get(config::get_config).put(config::set_config),
        )
        .route("/api/config/{key}", delete(config::reset_config))
        .route("/api/session", put(session::configure))
        .route("/api/watch", get(watch::watch))
        .route("/health", get(session::health))
        .route("/api/{*path}", any(|| async { StatusCode::NOT_FOUND }))
        .fallback(assets::handler)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            super::local_request::local_request,
        ))
        .with_state(state)
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

/// Run synchronous repository work off the async request thread.
pub(super) async fn away<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(e) => Err(Error::msg(format!("the request could not be run: {e}"))),
    }
}

/// Every failure here is the caller asking for something that is not under
/// review, which is a bad request rather than a broken server.
pub(super) fn fail(e: Error) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, e.to_string()).into_response()
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
    use crate::server::SessionConfig;
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
