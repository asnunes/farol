//! The list of files that can be read diagonally.
use super::*;

impl ReviewMap {
    pub fn add_skim(
        &mut self,
        path: impl Into<String>,
        reason: impl Into<String>,
        block: Option<Slug>,
    ) -> Result<()> {
        let path = path.into();
        if let Some(slug) = &block
            && self.index_of(slug).is_none()
        {
            return Err(MapError::UnknownBlock {
                slug: slug.to_string(),
                existing: self.slugs(),
            });
        }
        self.skim.retain(|s| s.path != path);
        self.skim.push(SkimEntry {
            path,
            reason: reason.into(),
            block,
        });
        Ok(())
    }

    pub fn remove_skim(&mut self, path: &str) -> Result<()> {
        let before = self.skim.len();
        self.skim.retain(|s| s.path != path);
        if self.skim.len() == before {
            return Err(MapError::NotSkimmed {
                path: path.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{map_with, slug};

    #[test]
    fn skim_cannot_point_at_a_block_that_does_not_exist() {
        let mut m = map_with(&["one"]);
        let err = m
            .add_skim("Cargo.lock", "generated", Some(slug("ghost")))
            .unwrap_err();
        assert!(matches!(err, MapError::UnknownBlock { .. }));
    }

    #[test]
    fn skim_splits_into_attached_and_loose() {
        // Both the renderer and the screen need this split; it lives here so
        // they cannot disagree about it.
        let mut m = map_with(&["one"]);
        m.add_skim("a_test.rs", "fixture only", Some(slug("one")))
            .unwrap();
        m.add_skim("Cargo.lock", "generated", None).unwrap();

        let attached: Vec<_> = m.skim_for(&slug("one")).map(|s| s.path.as_str()).collect();
        let loose: Vec<_> = m.loose_skim().map(|s| s.path.as_str()).collect();
        assert_eq!(attached, vec!["a_test.rs"]);
        assert_eq!(loose, vec!["Cargo.lock"]);
        assert_eq!(m.skim_for(&slug("ghost")).count(), 0);
    }
}
