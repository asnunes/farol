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
    scope: Scope,
    /// (from, to, path) -> the diff between those two commits.
    between: Vec<((String, String, String), FileDiff)>,
    line_counts: Vec<(String, u32)>,
    ancestors: Vec<String>,
    distances: Vec<(String, u32)>,
}

impl FakeDiffSource {
    pub fn with_paths(paths: &[&str]) -> Self {
        Self {
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
            hunks: vec![],
            binary: false,
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
}

impl ProgressRepository for InMemoryProgressRepository {
    fn load(&self) -> Result<Progress> {
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
