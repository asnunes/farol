use crate::diff::application::FileDiffs;
use crate::diff::domain::FileDiff;
use crate::error::Result;

/// One file's diff, for the pane on the right.
#[derive(Clone)]
pub struct GetFileDiff {
    diffs: FileDiffs,
}

impl GetFileDiff {
    pub fn new(diffs: FileDiffs) -> Self {
        Self { diffs }
    }

    pub fn execute(&self, path: &str) -> Result<FileDiff> {
        self.diffs.of(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeDiffSource;
    use std::sync::Arc;

    fn on(paths: &[&str]) -> GetFileDiff {
        GetFileDiff::new(FileDiffs::new(Arc::new(FakeDiffSource::with_paths(paths))))
    }

    #[test]
    fn a_file_under_review_comes_back_with_its_diff() {
        let diff = on(&["a.rs"]).execute("a.rs").unwrap();

        assert_eq!(diff.path, "a.rs");
        assert_eq!((diff.additions, diff.deletions), (3, 1));
    }

    #[test]
    fn a_file_outside_the_review_is_refused_rather_than_served() {
        // The pane is driven by the path in the URL; without this a crafted
        // request would read any file in the repository.
        let err = on(&["a.rs"]).execute("../../etc/passwd").unwrap_err();

        assert!(err.to_string().contains("passwd"), "{err}");
    }
}
