//! Opening, rewriting, removing and reordering blocks.
use super::*;

impl ReviewMap {
    pub fn add_block(
        &mut self,
        slug: &Slug,
        title: impl Into<String>,
        context: impl Into<String>,
        position: Position,
    ) -> Result<()> {
        if self.index_of(slug).is_some() {
            return Err(Error::DuplicateBlock {
                slug: slug.to_string(),
            });
        }
        let at = self.resolve_position(&position)?;
        self.blocks.insert(
            at,
            Block {
                slug: slug.clone(),
                title: title.into(),
                context: context.into(),
                files: Vec::new(),
            },
        );
        Ok(())
    }

    pub fn update_block(
        &mut self,
        slug: &Slug,
        title: Option<String>,
        context: Option<String>,
    ) -> Result<()> {
        let block = self.block_mut(slug)?;
        if let Some(t) = title {
            block.title = t;
        }
        if let Some(c) = context {
            block.context = c;
        }
        Ok(())
    }

    /// Removing a block orphans its line notes rather than dropping them: the
    /// prose may still be worth moving somewhere else.
    pub fn remove_block(&mut self, slug: &Slug) -> Result<()> {
        let idx = self.index_of(slug).ok_or_else(|| Error::UnknownBlock {
            slug: slug.to_string(),
            existing: self.slugs(),
        })?;
        let block = self.blocks.remove(idx);
        for file in &block.files {
            for note in &file.line_notes {
                self.orphans.push(Orphan {
                    block: block.slug.clone(),
                    path: file.path.clone(),
                    old_range: note.range,
                    snapshot: String::new(),
                    reason: OrphanReason::BlockRemoved,
                    text: note.text.clone(),
                });
            }
        }
        for entry in &mut self.skim {
            if entry.block.as_ref() == Some(slug) {
                entry.block = None;
            }
        }
        Ok(())
    }

    pub fn move_block(&mut self, slug: &Slug, position: Position) -> Result<()> {
        let idx = self.index_of(slug).ok_or_else(|| Error::UnknownBlock {
            slug: slug.to_string(),
            existing: self.slugs(),
        })?;
        let block = self.blocks.remove(idx);
        let at = match self.resolve_position(&position) {
            Ok(at) => at,
            Err(e) => {
                self.blocks.insert(idx, block);
                return Err(e);
            }
        };
        self.blocks.insert(at, block);
        Ok(())
    }

    fn resolve_position(&self, position: &Position) -> Result<usize> {
        match position {
            Position::End => Ok(self.blocks.len()),
            Position::Before(target) => self.index_of(target).ok_or_else(|| Error::UnknownBlock {
                slug: target.to_string(),
                existing: self.slugs(),
            }),
            Position::After(target) => {
                self.index_of(target)
                    .map(|i| i + 1)
                    .ok_or_else(|| Error::UnknownBlock {
                        slug: target.to_string(),
                        existing: self.slugs(),
                    })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::slug;

    fn map_with(slugs: &[&str]) -> ReviewMap {
        let mut m = ReviewMap::new("feature/x", "main", "abc123");
        for s in slugs {
            m.add_block(&slug(s), "t", "c", Position::End).unwrap();
        }
        m
    }

    #[test]
    fn blocks_keep_insertion_order() {
        let m = map_with(&["one", "two", "three"]);
        assert_eq!(m.slugs(), vec!["one", "two", "three"]);
    }

    #[test]
    fn duplicate_slug_is_rejected() {
        let mut m = map_with(&["one"]);
        let err = m
            .add_block(&slug("one"), "t", "c", Position::End)
            .unwrap_err();
        assert!(matches!(err, Error::DuplicateBlock { .. }));
    }

    #[test]
    fn add_before_lands_ahead_of_the_target() {
        let mut m = map_with(&["one", "two"]);
        m.add_block(&slug("mid"), "t", "c", Position::Before(slug("two")))
            .unwrap();
        assert_eq!(m.slugs(), vec!["one", "mid", "two"]);
    }

    #[test]
    fn add_after_lands_behind_the_target() {
        let mut m = map_with(&["one", "two"]);
        m.add_block(&slug("mid"), "t", "c", Position::After(slug("one")))
            .unwrap();
        assert_eq!(m.slugs(), vec!["one", "mid", "two"]);
    }

    #[test]
    fn add_before_the_first_block_reaches_the_top() {
        let mut m = map_with(&["one", "two"]);
        m.add_block(&slug("zero"), "t", "c", Position::Before(slug("one")))
            .unwrap();
        assert_eq!(m.slugs(), vec!["zero", "one", "two"]);
    }

    #[test]
    fn positioning_against_an_unknown_slug_fails_and_lists_the_real_ones() {
        let mut m = map_with(&["one", "two"]);
        let err = m
            .add_block(&slug("x"), "t", "c", Position::After(slug("nope")))
            .unwrap_err();
        match err {
            Error::UnknownBlock { existing, .. } => assert_eq!(existing, vec!["one", "two"]),
            other => panic!("unexpected: {other:?}"),
        }
        assert_eq!(m.slugs(), vec!["one", "two"], "map must be untouched");
    }

    #[test]
    fn moving_a_block_to_the_front_and_to_the_back() {
        let mut m = map_with(&["one", "two", "three"]);
        m.move_block(&slug("three"), Position::Before(slug("one")))
            .unwrap();
        assert_eq!(m.slugs(), vec!["three", "one", "two"]);
        m.move_block(&slug("three"), Position::End).unwrap();
        assert_eq!(m.slugs(), vec!["one", "two", "three"]);
    }

    #[test]
    fn failed_move_leaves_the_block_where_it_was() {
        let mut m = map_with(&["one", "two"]);
        let err = m
            .move_block(&slug("one"), Position::After(slug("ghost")))
            .unwrap_err();
        assert!(matches!(err, Error::UnknownBlock { .. }));
        assert_eq!(m.slugs(), vec!["one", "two"]);
    }

    #[test]
    fn removing_a_block_keeps_its_line_notes_as_orphans() {
        let mut m = map_with(&["one"]);
        m.add_file(&slug("one"), "a.rs", None, None).unwrap();
        m.add_line_note(
            &slug("one"),
            "a.rs",
            LineRange::new(10, 20).unwrap(),
            "worth keeping",
        )
        .unwrap();
        m.remove_block(&slug("one")).unwrap();
        assert_eq!(m.orphans.len(), 1);
        assert_eq!(m.orphans[0].reason, OrphanReason::BlockRemoved);
        assert_eq!(m.orphans[0].text, "worth keeping");
    }

    #[test]
    fn removing_a_block_detaches_skim_entries_that_pointed_at_it() {
        let mut m = map_with(&["one"]);
        m.add_skim("go.sum", "generated", Some(slug("one")))
            .unwrap();
        m.remove_block(&slug("one")).unwrap();
        assert_eq!(m.skim[0].block, None);
    }
}
