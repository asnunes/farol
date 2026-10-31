//! One review, wired the way the use cases on threads and pending comments
//! need it, so each test says only what is particular to it.

use std::sync::Arc;

use crate::comments::application::{
    CloseThread, CommentReach, Comments, EditPending, GeneratePending, PublishReview,
    PullRequestGate, RemovePending, ReviewModes, WritePending,
};
use crate::comments::domain::Comment;
use crate::comments::infra::MarkdownComments;
use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
use crate::diff::domain::Side;
use crate::map::application::GetScope;
use crate::progress::application::ProgressStore;
use crate::shared::paths::Store;
use crate::testing::{
    FakeDiffSource, FakePublisher, InMemoryConfigRepository, InMemoryModeStore,
    InMemoryProgressRepository, RecordingChannel,
};

pub struct World {
    _dir: tempfile::TempDir,
    pub comments: Comments,
    pub publisher: Arc<FakePublisher>,
    pub channel: Arc<RecordingChannel>,
    pub config: Arc<InMemoryConfigRepository>,
    pub modes: ReviewModes,
    pub progress: ProgressStore,
    source: Arc<FakeDiffSource>,
}

impl World {
    /// A pull request by somebody else, showing the commit under review.
    pub fn ready() -> Self {
        Self::with(FakePublisher::ready_at("head"))
    }

    pub fn with(publisher: FakePublisher) -> Self {
        Self::over(publisher, a_file())
    }

    pub fn over(publisher: FakePublisher, source: FakeDiffSource) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path(), "feature/x");
        let source = Arc::new(source);
        let scope = ReviewScope::new(source.clone());
        let comments = Comments::new(
            Arc::new(MarkdownComments::new(&store, dir.path())),
            GetScope::new(scope.clone()),
            CommentReach::new(scope, FileDiffs::new(source.clone())),
        );
        Self {
            _dir: dir,
            comments,
            publisher: Arc::new(publisher),
            channel: Arc::new(RecordingChannel::default()),
            config: Arc::new(InMemoryConfigRepository::default()),
            modes: ReviewModes::new(Arc::new(InMemoryModeStore::default())),
            progress: ProgressStore::new(
                Arc::new(InMemoryProgressRepository::default()),
                FileDiffs::new(source.clone()),
            ),
            source,
        }
    }

    pub fn thread(&self) -> Comment {
        self.comments
            .add("src/a.rs", Side::New, 10, 12, "Why this order?")
            .unwrap()
    }

    pub fn gate(&self) -> PullRequestGate {
        PullRequestGate::new(
            self.publisher.clone(),
            ReviewScope::new(self.source.clone()),
            CommitHistory::new(self.source.clone()),
        )
    }

    pub fn write_pending(&self) -> WritePending {
        WritePending::new(self.comments.clone(), self.gate(), self.publisher.clone())
    }

    pub fn edit_pending(&self) -> EditPending {
        EditPending::new(self.publisher.clone())
    }

    pub fn remove_pending(&self) -> RemovePending {
        RemovePending::new(self.comments.clone(), self.publisher.clone())
    }

    pub fn close_thread(&self) -> CloseThread {
        CloseThread::new(self.comments.clone())
    }

    pub fn generate_pending(&self) -> GeneratePending {
        GeneratePending::new(
            self.comments.clone(),
            Some(self.channel.clone()),
            self.config.clone(),
            self.modes.clone(),
            self.publisher.clone(),
        )
    }

    pub fn publish_review(&self) -> PublishReview {
        PublishReview::new(
            self.comments.clone(),
            self.gate(),
            self.publisher.clone(),
            ReviewScope::new(self.source.clone()),
            self.read_files(),
        )
    }

    /// What the reviewer read, over a review with no map: the ticks alone.
    fn read_files(&self) -> crate::lenses::application::ListReadFiles {
        let scope = ReviewScope::new(self.source.clone());
        let versions = crate::map::application::MapVersions::new(
            scope.clone(),
            CommitHistory::new(self.source.clone()),
            Arc::new(crate::testing::InMemoryMapRepository::default()),
        );
        crate::lenses::application::ListReadFiles::new(
            self.progress.clone(),
            scope.clone(),
            crate::lenses::application::GetReviewWithLenses::new(
                crate::map::application::GetReview::new(versions, scope, self.progress.clone()),
                Arc::new(crate::testing::InMemoryLensCatalog::default()),
                Arc::new(crate::testing::InMemoryLensAnswers::default()),
            ),
        )
    }
}

/// One 200-line file whose diff prints lines 10–14 and 40–42.
fn a_file() -> FakeDiffSource {
    FakeDiffSource::with_paths(&["src/a.rs"])
        .with_line_count("src/a.rs", 200)
        .showing("src/a.rs", vec![hunk(10, 5), hunk(40, 3)])
}

pub fn hunk(start: u32, lines: u32) -> crate::diff::domain::Hunk {
    crate::diff::domain::Hunk {
        old_start: start,
        old_lines: lines,
        new_start: start,
        new_lines: lines,
        lines: vec![],
    }
}
