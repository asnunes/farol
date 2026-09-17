//! Stand-ins for the ports, so behaviour can be driven without a repository on
//! disk.
//!
//! This is what the injection buys: derivation, re-anchoring and the view rules
//! are exercised against hand-built diffs, where a case that would take three
//! commits to set up is four lines instead.

use std::sync::{Arc, Mutex};

use crate::diff::domain::{
    CommitHistorySource, FileChange, FileDiff, FileDiffSource, FileStatus, Hunk, Line, LineKind,
    ReviewScopeSource, Scope,
};
use crate::error::Result;
use crate::map::domain::{MapRepository, ReviewMap, Slug};
use crate::progress::domain::{Progress, ProgressRepository};

/// A diff source you assemble by hand.
pub struct FakeDiffSource {
    head_reads: Mutex<std::collections::VecDeque<Result<crate::diff::domain::HeadState>>>,
    scope: Scope,
    /// (from, to, path) -> the diff between those two commits.
    between: Vec<((String, String, String), FileDiff)>,
    line_counts: Vec<(String, u32)>,
    /// path -> the hunks its diff prints, when a test cares which.
    shown: Vec<(String, Vec<Hunk>)>,
    ancestors: Vec<String>,
    distances: Vec<(String, u32)>,
}

impl FakeDiffSource {
    pub fn with_head_reads(self, reads: Vec<Result<crate::diff::domain::HeadState>>) -> Self {
        *self.head_reads.lock().unwrap() = reads.into();
        self
    }

    pub fn with_paths(paths: &[&str]) -> Self {
        Self {
            head_reads: Mutex::new(Default::default()),
            scope: Scope {
                branch: "feature/x".into(),
                base_ref: "main".into(),
                head_ref: "feature/x".into(),
                base_sha: "base".into(),
                head_sha: "head".into(),
                merge_base: true,
                dirty: false,
                files: paths.iter().map(|p| change(p)).collect(),
            },
            between: Vec::new(),
            line_counts: Vec::new(),
            shown: Vec::new(),
            ancestors: Vec::new(),
            distances: Vec::new(),
        }
    }

    /// Declare how a file changed between two commits, which is what
    /// re-anchoring reads.
    pub fn changed_between(mut self, from: &str, to: &str, path: &str, hunks: Vec<Hunk>) -> Self {
        self.between.push((
            (from.into(), to.into(), path.into()),
            FileDiff {
                path: path.into(),
                old_path: None,
                status: FileStatus::Modified,
                hunks,
                binary: false,
                additions: 0,
                deletions: 0,
                line_count: 0,
                new_content_hash: format!("hash-of-{path}-at-{to}"),
            },
        ));
        self
    }

    /// As if `--dirty` were given, so the working-tree marker is the target.
    pub fn dirty(mut self) -> Self {
        self.scope.dirty = true;
        self
    }

    /// Declare which hunks the file's diff prints, for a test about what the
    /// diff reaches rather than about what the file contains.
    pub fn showing(mut self, path: &str, hunks: Vec<Hunk>) -> Self {
        self.shown.push((path.into(), hunks));
        self
    }

    pub fn with_line_count(mut self, path: &str, lines: u32) -> Self {
        self.line_counts.push((path.into(), lines));
        self
    }

    pub fn with_ancestors(mut self, shas: &[&str]) -> Self {
        self.ancestors = shas.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn at_distance(mut self, sha: &str, commits: u32) -> Self {
        self.distances.push((sha.into(), commits));
        self
    }

    /// The commit the review is measured from. What an export is matched by.
    pub fn from_base(mut self, sha: &str) -> Self {
        self.scope.base_sha = sha.into();
        self
    }

    pub fn on_commit(mut self, sha: &str) -> Self {
        self.scope.head_sha = sha.into();
        self
    }
}

fn change(path: &str) -> FileChange {
    FileChange {
        path: path.to_string(),
        old_path: None,
        status: FileStatus::Modified,
        additions: 3,
        deletions: 1,
        // The same identity `content_hash` hands out, so a fake scope and a
        // fake mark of `read` agree with each other.
        content_hash: format!("hash-of-{path}"),
    }
}

impl FakeDiffSource {
    /// What the file's diff prints. Undeclared, it prints the whole file —
    /// generous like `file_line_count` is, so a test only says where the diff
    /// reaches when that is the thing under test.
    fn hunks_of(&self, path: &str) -> Result<Vec<Hunk>> {
        if let Some((_, hunks)) = self.shown.iter().find(|(p, _)| p == path) {
            return Ok(hunks.clone());
        }
        let lines = self.file_line_count(path)?;
        Ok(vec![Hunk {
            old_start: 1,
            old_lines: lines,
            new_start: 1,
            new_lines: lines,
            lines: vec![],
        }])
    }
}

impl ReviewScopeSource for FakeDiffSource {
    fn scope(&self) -> Result<&Scope> {
        Ok(&self.scope)
    }

    fn file_line_count(&self, path: &str) -> Result<u32> {
        if !self.scope.contains(path) {
            return Err(self.scope.reject(path).into());
        }
        // Generous by default so a test only declares a size when the size is
        // the thing under test.
        Ok(self
            .line_counts
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, n)| *n)
            .unwrap_or(10_000))
    }

    /// The line at `n` says which line it is, so a test that opens a range can
    /// assert it got that range and not one beside it.
    fn file_lines(&self, path: &str, from: u32, to: u32) -> Result<Vec<String>> {
        let count = self.file_line_count(path)?;
        let last = to.min(count);
        Ok((from.max(1)..=last).map(|n| format!("line {n}")).collect())
    }
}

impl FileDiffSource for FakeDiffSource {
    fn file_diff(&self, path: &str) -> Result<FileDiff> {
        // The real source refuses a path outside the window; a fake that did
        // not would let tests pass over behaviour that does not exist.
        if !self.scope.contains(path) {
            return Err(self.scope.reject(path).into());
        }
        Ok(FileDiff {
            path: path.to_string(),
            old_path: None,
            status: FileStatus::Modified,
            hunks: self.hunks_of(path)?,
            binary: false,
            line_count: self.file_line_count(path)?,
            additions: 3,
            deletions: 1,
            new_content_hash: format!("hash-of-{path}"),
        })
    }

    fn content_hash(&self, path: &str) -> Result<String> {
        // The real source refuses a path outside the window; so does this.
        if !self.scope.contains(path) {
            return Err(self.scope.reject(path).into());
        }
        Ok(format!("hash-of-{path}"))
    }

    fn file_diff_between(&self, from: &str, to: &str, path: &str) -> Result<Option<FileDiff>> {
        Ok(self
            .between
            .iter()
            .find(|((f, t, p), _)| f == from && t == to && p == path)
            .map(|(_, diff)| diff.clone()))
    }
}

impl CommitHistorySource for FakeDiffSource {
    fn head_sha(&self) -> Result<String> {
        Ok(self.scope.head_sha.clone())
    }

    fn commits_ahead_of(&self, sha: &str) -> Result<u32> {
        Ok(self
            .distances
            .iter()
            .find(|(s, _)| s == sha)
            .map(|(_, n)| *n)
            .unwrap_or(0))
    }

    fn is_ancestor(&self, sha: &str) -> Result<bool> {
        Ok(self.ancestors.iter().any(|s| s == sha))
    }
}

/// Map storage that never touches the filesystem.
#[derive(Default)]
pub struct InMemoryMapRepository {
    maps: Mutex<Vec<ReviewMap>>,
}

impl InMemoryMapRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, map: ReviewMap) {
        self.maps.lock().unwrap().push(map);
    }
}

impl MapRepository for InMemoryMapRepository {
    fn load_at(&self, sha: &str) -> Result<Option<ReviewMap>> {
        Ok(self
            .maps
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.generated_at == sha)
            .cloned())
    }

    fn stored_shas(&self) -> Result<Vec<String>> {
        Ok(self
            .maps
            .lock()
            .unwrap()
            .iter()
            .map(|m| m.generated_at.clone())
            .collect())
    }

    fn save(&self, map: &ReviewMap) -> Result<()> {
        let mut maps = self.maps.lock().unwrap();
        maps.retain(|m| m.generated_at != map.generated_at);
        maps.push(map.clone());
        Ok(())
    }

    fn delete(&self, sha: &str) -> Result<()> {
        self.maps.lock().unwrap().retain(|m| m.generated_at != sha);
        Ok(())
    }
}

/// Storage that refuses to answer, for the paths that only run when something
/// underneath has broken.
#[derive(Default)]
pub struct BrokenProgressRepository;

impl ProgressRepository for BrokenProgressRepository {
    fn load(&self) -> Result<Progress> {
        Err(crate::error::Error::msg("the store is unreadable"))
    }

    fn save(&self, _progress: &Progress) -> Result<()> {
        Err(crate::error::Error::msg("the store is unwritable"))
    }
}

#[derive(Default)]
pub struct InMemoryProgressRepository {
    progress: Mutex<Progress>,
    /// A store that cannot be read, for the tests about what happens to work
    /// that depends on knowing what was read.
    broken: bool,
}

impl InMemoryProgressRepository {
    pub fn broken() -> Self {
        Self {
            broken: true,
            ..Default::default()
        }
    }
}

impl ProgressRepository for InMemoryProgressRepository {
    fn load(&self) -> Result<Progress> {
        if self.broken {
            return Err(crate::error::Error::msg("progress file is not readable"));
        }
        Ok(self.progress.lock().unwrap().clone())
    }

    fn save(&self, progress: &Progress) -> Result<()> {
        *self.progress.lock().unwrap() = progress.clone();
        Ok(())
    }
}

/// A hunk with only the counts filled in — enough for range arithmetic.
pub fn hunk(old_start: u32, old_lines: u32, new_lines: u32) -> Hunk {
    Hunk {
        old_start,
        old_lines,
        new_start: old_start,
        new_lines,
        lines: vec![],
    }
}

/// A hunk that also carries line bodies, so snapshots have something to quote.
pub fn hunk_with_lines(old_start: u32, contents: &[&str]) -> Hunk {
    Hunk {
        old_start,
        old_lines: contents.len() as u32,
        new_start: old_start,
        new_lines: contents.len() as u32,
        lines: contents
            .iter()
            .enumerate()
            .map(|(i, c)| Line {
                kind: LineKind::Removed,
                old_number: Some(old_start + i as u32),
                new_number: None,
                content: c.to_string(),
            })
            .collect(),
    }
}

/// A slug from a literal. Tests state the shape they mean; a malformed one is a
/// bug in the test, not a case under test.
pub fn slug(s: &str) -> Slug {
    Slug::parse(s).expect("test slugs must be well formed")
}

/// The map services over fakes, assembled the way the composition root does.
///
/// Returned together because a test that writes usually also reads back, and
/// splitting the call would make every test build both by hand.
pub struct MapServices {
    pub versions: crate::map::application::MapVersions,
    pub derivation: crate::map::application::MapDerivation,
    pub editor: crate::map::application::MapEditor,
}

pub fn services(source: FakeDiffSource, repo: Arc<InMemoryMapRepository>) -> MapServices {
    use crate::diff::application::{CommitHistory, FileDiffs, ReviewScope};
    use crate::map::application::{MapDerivation, MapEditor, MapReconciler, MapVersions};

    let source = Arc::new(source);
    let scope = ReviewScope::new(source.clone());
    let reconciler = MapReconciler::new(scope.clone(), FileDiffs::new(source.clone()));
    let versions = MapVersions::new(scope.clone(), CommitHistory::new(source), repo.clone());
    let derivation = MapDerivation::new(versions.clone(), scope, reconciler, repo.clone());

    MapServices {
        editor: MapEditor::new(derivation.clone(), versions.clone(), repo),
        versions,
        derivation,
    }
}

/// The editor alone, for the many tests that only write.
pub fn editor(
    source: FakeDiffSource,
    repo: Arc<InMemoryMapRepository>,
) -> crate::map::application::MapEditor {
    services(source, repo).editor
}

/// The reconciler over fakes, for exercising note movement directly instead of
/// through a derivation.
pub fn reconciler(source: FakeDiffSource) -> crate::map::application::MapReconciler {
    use crate::diff::application::{FileDiffs, ReviewScope};
    let source = Arc::new(source);
    crate::map::application::MapReconciler::new(
        ReviewScope::new(source.clone()),
        FileDiffs::new(source),
    )
}

/// A map holding one empty block per slug, in the order given.
///
/// The fixture the map's own tests are written against: five of them had
/// written it out, identically, before it moved here.
pub fn map_with(slugs: &[&str]) -> crate::map::domain::ReviewMap {
    use crate::map::domain::{Position, ReviewMap};
    let mut map = ReviewMap::new("feature/x", "main", "abc123");
    for s in slugs {
        map.add_block(&slug(s), "t", "c", Position::End).unwrap();
    }
    map
}

/// A map with one block, one file and one note on it.
pub fn map_with_note(
    sha: &str,
    range: crate::map::domain::LineRange,
    text: &str,
) -> crate::map::domain::ReviewMap {
    use crate::map::domain::{Position, ReviewMap};
    let mut map = ReviewMap::new("feature/x", "main", sha);
    map.add_block(&slug("core"), "Core", "why", Position::End)
        .unwrap();
    map.add_file(&slug("core"), "a.rs", None, None).unwrap();
    map.add_line_note(&slug("core"), "a.rs", range, text)
        .unwrap();
    map
}

/// A map service and a scope over the same fake, for exercising a use case the
/// way `wiring` assembles it.
pub fn use_case_setup(paths: &[&str]) -> (MapServices, crate::diff::application::ReviewScope) {
    // A declared size, so a range check has something definite to fail against.
    let fake = || {
        paths
            .iter()
            .fold(FakeDiffSource::with_paths(paths), |f, p| {
                f.with_line_count(p, 100)
            })
    };
    let scope = crate::diff::application::ReviewScope::new(Arc::new(fake()));
    (
        services(fake(), Arc::new(InMemoryMapRepository::new())),
        scope,
    )
}

/// A block called `core` holding the given paths, which is the arrangement
/// almost every use-case test needs before it can do anything interesting.
pub fn with_block(paths: &[&str]) -> (MapServices, crate::diff::application::ReviewScope) {
    let (svc, scope) = use_case_setup(paths);
    let resolved = scope
        .paths(&paths.iter().map(|p| p.to_string()).collect::<Vec<_>>())
        .expect("the fake serves every path it was given");
    crate::map::application::AddBlock::new(svc.editor.clone())
        .execute(
            &slug("core"),
            "t",
            "c",
            crate::map::domain::Position::End,
            &resolved,
        )
        .expect("opening the first block cannot fail");
    (svc, scope)
}

pub fn range(from: u32, to: u32) -> crate::map::domain::LineRange {
    crate::map::domain::LineRange::new(from, to).expect("test ranges must be well formed")
}

/// Paths in `core`, in reading order.
pub fn block_paths(map: &crate::map::domain::ReviewMap) -> Vec<String> {
    map.block(&slug("core"))
        .expect("the core block should be there")
        .files
        .iter()
        .map(|f| f.path.clone())
        .collect()
}

/// Line notes on `core`'s first file, in the order they will be read.
pub fn line_notes(
    map: &crate::map::domain::ReviewMap,
) -> Vec<(crate::map::domain::LineRange, String)> {
    map.block(&slug("core"))
        .expect("the core block should be there")
        .files
        .first()
        .map(|f| {
            f.line_notes
                .iter()
                .map(|n| (n.range, n.text.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// The state a derivation leaves behind when a note's code was rewritten.
///
/// Goes through the same path a real derivation takes — write the note, then
/// let the map deactivate it — rather than reaching into the map, which is no
/// longer possible and was never a fair setup.
pub fn orphaned(
    editor: &crate::map::application::MapEditor,
    old: crate::map::domain::LineRange,
    text: &str,
) {
    use crate::map::domain::{MapError, NoteFate, OrphanReason};

    editor
        .edit(|map| {
            map.add_line_note(&slug("core"), "a.rs", old, text)?;
            map.reanchor_notes(|_, note| {
                Ok::<_, MapError>(if note.range == old {
                    NoteFate::Orphan {
                        snapshot: "the code it covered".into(),
                        reason: OrphanReason::HunkOverlap,
                    }
                } else {
                    NoteFate::Keep
                })
            })
        })
        .expect("seeding an orphan cannot fail");
}

/// A publisher that keeps what it was handed instead of sending it.
///
/// How the assembled review is asserted without a network — and, in the server
/// tests, how a route reaches every state of readiness without one either.
pub struct FakePublisher {
    readiness: crate::comments::domain::Readiness,
    sent: Mutex<Option<crate::comments::domain::Review>>,
    marked: Mutex<Vec<String>>,
    /// What `mark_read` answers. `Err` is the case that matters: the review is
    /// already on the pull request when it happens.
    marking: Mutex<Option<String>>,
}

impl FakePublisher {
    pub fn ready_at(head: &str) -> Self {
        Self::blocked(crate::comments::domain::Readiness::Ready {
            pull_request: 12,
            id: "PR_kwDO".into(),
            mine: false,
            head: head.into(),
        })
    }

    pub fn blocked(readiness: crate::comments::domain::Readiness) -> Self {
        Self {
            readiness,
            sent: Mutex::new(None),
            marked: Mutex::new(Vec::new()),
            marking: Mutex::new(None),
        }
    }

    /// The pull request belongs to whoever is reviewing it, which is the one
    /// arrangement where approving is not on the table.
    pub fn mine(mut self) -> Self {
        if let crate::comments::domain::Readiness::Ready { mine, .. } = &mut self.readiness {
            *mine = true;
        }
        self
    }

    /// A host that takes the review and then refuses to mark anything.
    pub fn marking_fails(self, why: &str) -> Self {
        *self.marking.lock().unwrap() = Some(why.into());
        self
    }

    pub fn marked(&self) -> Vec<String> {
        self.marked.lock().unwrap().clone()
    }

    pub fn sent(&self) -> crate::comments::domain::Review {
        self.sent.lock().unwrap().clone().expect("nothing was sent")
    }

    pub fn nothing_sent(&self) -> bool {
        self.sent.lock().unwrap().is_none()
    }
}

impl crate::comments::domain::ReviewPublisher for FakePublisher {
    fn readiness(&self, _branch: &str) -> Result<crate::comments::domain::Readiness> {
        Ok(self.readiness.clone())
    }

    fn publish(&self, review: &crate::comments::domain::Review) -> Result<String> {
        *self.sent.lock().unwrap() = Some(review.clone());
        Ok("https://example.test/r1".into())
    }

    fn mark_read(&self, _pull: &str, paths: &[String]) -> Result<usize> {
        if let Some(why) = self.marking.lock().unwrap().clone() {
            return Err(crate::comments::domain::CommentError::Refused { what: why }.into());
        }
        self.marked.lock().unwrap().extend_from_slice(paths);
        Ok(paths.len())
    }
}

/// Host-scoped credentials for publisher tests; never consults the real gh login.
#[derive(Default)]
pub struct FakeCredentials {
    token: Mutex<Option<(String, String)>>,
}

impl FakeCredentials {
    pub fn set(&self, host: &str, token: &str) -> Result<()> {
        *self.token.lock().unwrap() = Some((host.to_string(), token.to_string()));
        Ok(())
    }
}

impl crate::comments::domain::Credentials for FakeCredentials {
    fn token(&self, host: &str) -> Result<Option<String>> {
        Ok(self
            .token
            .lock()
            .unwrap()
            .as_ref()
            .filter(|(saved, _)| saved == host)
            .map(|(_, token)| token.clone()))
    }
}

/// Comments in memory, for the use cases that only care what is in the store.
#[derive(Default)]
pub struct InMemoryComments {
    comments: Mutex<Vec<crate::comments::domain::Comment>>,
}

impl crate::comments::domain::CommentStore for InMemoryComments {
    fn list(&self) -> Result<crate::comments::domain::Found> {
        Ok(crate::comments::domain::Found {
            comments: self.comments.lock().unwrap().clone(),
            unreadable: vec![],
        })
    }

    fn save(&self, comment: &crate::comments::domain::Comment) -> Result<()> {
        let mut all = self.comments.lock().unwrap();
        match all.iter().position(|c| c.id == comment.id) {
            Some(at) => all[at] = comment.clone(),
            None => all.push(comment.clone()),
        }
        Ok(())
    }

    fn close(&self, id: &str) -> Result<bool> {
        let mut all = self.comments.lock().unwrap();
        let before = all.len();
        all.retain(|c| c.id != id);
        Ok(all.len() != before)
    }
}
pub struct FixedServerUseCases(pub crate::cmd::ServerUseCases);

impl crate::cmd::ServerUseCaseFactory for FixedServerUseCases {
    fn build(
        &self,
        _: crate::diff::infra::ScopeRequest,
    ) -> crate::error::Result<crate::cmd::ServerUseCases> {
        Ok(self.0.clone())
    }
}

impl crate::diff::domain::HeadSource for FakeDiffSource {
    fn read_head(&self) -> Result<crate::diff::domain::HeadState> {
        if let Some(read) = self.head_reads.lock().unwrap().pop_front() {
            return read;
        }
        Ok(crate::diff::domain::HeadState {
            reference: Some(format!("refs/heads/{}", self.scope.branch)),
            commit: self.scope.head_sha.clone(),
        })
    }
}
