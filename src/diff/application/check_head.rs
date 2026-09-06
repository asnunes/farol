use std::sync::Arc;

use crate::diff::domain::{HeadSource, HeadState};
use crate::error::Result;

/// Filesystem events only hint at change; Git establishes whether HEAD moved.
pub struct CheckHead {
    source: Arc<dyn HeadSource>,
    previous: Option<HeadState>,
}

impl CheckHead {
    pub fn new(source: Arc<dyn HeadSource>) -> Self {
        Self {
            source,
            previous: None,
        }
    }

    pub fn execute(&mut self) -> Result<bool> {
        let current = self.source.read_head()?;
        let changed = self
            .previous
            .as_ref()
            .is_some_and(|previous| previous != &current);
        self.previous = Some(current);
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeDiffSource;

    #[test]
    fn only_a_changed_commit_or_branch_is_news() {
        let state = |reference: &str, commit: &str| {
            Ok(HeadState {
                reference: Some(reference.into()),
                commit: commit.into(),
            })
        };
        let source = FakeDiffSource::with_paths(&[]).with_head_reads(vec![
            state("refs/heads/main", "a"),
            state("refs/heads/main", "a"),
            state("refs/heads/main", "b"),
            state("refs/heads/feature", "b"),
            Err(crate::error::Error::msg("cannot read HEAD")),
            state("refs/heads/feature", "b"),
            state("refs/heads/feature", "c"),
        ]);
        let mut check = CheckHead::new(Arc::new(source));
        assert!(
            !check.execute().unwrap(),
            "the first observation establishes a baseline"
        );
        assert!(
            !check.execute().unwrap(),
            "rewriting an unchanged reference is not news"
        );
        assert!(check.execute().unwrap(), "a new commit changes the review");
        assert!(
            check.execute().unwrap(),
            "another branch has its own map even at the same commit"
        );
        assert!(check.execute().is_err());
        assert!(
            !check.execute().unwrap(),
            "a failed read must preserve the last valid observation"
        );
        assert!(check.execute().unwrap());
    }

    #[test]
    fn a_failed_initial_read_does_not_invent_a_change() {
        let source = FakeDiffSource::with_paths(&[])
            .with_head_reads(vec![Err(crate::error::Error::msg("cannot read HEAD"))]);
        let mut check = CheckHead::new(Arc::new(source));
        assert!(check.execute().is_err());
        assert!(!check.execute().unwrap());
    }
}
