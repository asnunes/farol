use std::sync::Arc;

use crate::diff::application::FileDiffs;
use crate::error::Result;
use crate::progress::domain::{Progress, ProgressRepository};

/// A **service**: the dependency the use cases share. Not called by transports.
#[derive(Clone)]
pub struct ProgressStore {
    repo: Arc<dyn ProgressRepository>,
    /// Only for the content hash a tick is pinned to — this store has no
    /// business walking history or resolving scope.
    diffs: FileDiffs,
}

impl ProgressStore {
    pub fn new(repo: Arc<dyn ProgressRepository>, diffs: FileDiffs) -> Self {
        Self { repo, diffs }
    }

    pub fn load(&self) -> Result<Progress> {
        self.repo.load()
    }

    /// Only current, in-scope marks can be offered for GitHub synchronization.
    pub fn current_paths(&self, files: &[crate::diff::domain::FileChange]) -> Result<Vec<String>> {
        let progress = self.load()?;
        let mut paths = Vec::new();
        for file in files {
            if progress.is_current(&file.path, &self.content_hash(&file.path)?) {
                paths.push(file.path.clone());
            }
        }
        Ok(paths)
    }

    pub fn save(&self, progress: &Progress) -> Result<()> {
        self.repo.save(progress)
    }

    pub(super) fn content_hash(&self, path: &str) -> Result<String> {
        self.diffs.content_hash(path)
    }
}

/// A store over this source, and the repository behind it for a test to read
/// what was written.
#[cfg(test)]
pub(super) fn store_over(
    source: crate::testing::FakeDiffSource,
) -> (
    ProgressStore,
    Arc<crate::testing::InMemoryProgressRepository>,
) {
    let repo = Arc::new(crate::testing::InMemoryProgressRepository::default());
    (
        ProgressStore::new(repo.clone(), FileDiffs::new(Arc::new(source))),
        repo,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeDiffSource;

    #[test]
    fn publication_marks_exclude_stale_and_out_of_scope_files() {
        let source = FakeDiffSource::with_paths(&["current.rs", "changed.rs"]);
        let scope =
            crate::diff::application::ReviewScope::new(Arc::new(FakeDiffSource::with_paths(&[
                "current.rs",
                "changed.rs",
            ])));
        let (store, repo) = store_over(source);
        let mut progress = Progress::default();
        progress.mark("current.rs", "hash-of-current.rs", "now");
        progress.mark("changed.rs", "old-content", "before");
        progress.mark("gone.rs", "hash-of-gone.rs", "before");
        repo.save(&progress).unwrap();
        assert_eq!(
            store.current_paths(&scope.get().unwrap().files).unwrap(),
            vec!["current.rs"]
        );
    }
}
