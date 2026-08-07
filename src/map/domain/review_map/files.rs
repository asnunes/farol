//! Putting files into a block and taking them out.
use super::*;

impl ReviewMap {
    pub fn add_file(
        &mut self,
        slug: &Slug,
        path: impl Into<String>,
        note: Option<String>,
        after: Option<&str>,
    ) -> Result<()> {
        let path = path.into();
        let block = self.block_mut(slug)?;

        if block.file(&path).is_some() {
            return Err(Error::DuplicatePath {
                slug: slug.to_string(),
                path,
            });
        }

        let at = match after {
            None => block.files.len(),
            Some(target) => block
                .files
                .iter()
                .position(|f| f.path == target)
                .map(|i| i + 1)
                .ok_or_else(|| Error::PathNotInBlock {
                    slug: slug.to_string(),
                    path: target.to_string(),
                    existing: block.paths(),
                })?,
        };
        block.files.insert(at, BlockFile::new(path, note));
        Ok(())
    }

    pub fn update_file(&mut self, slug: &Slug, path: &str, note: Option<String>) -> Result<()> {
        let block = self.block_mut(slug)?;
        let paths = block.paths();
        let file = block.file_mut(path).ok_or_else(|| Error::PathNotInBlock {
            slug: slug.to_string(),
            path: path.to_string(),
            existing: paths,
        })?;
        file.note = note;
        Ok(())
    }

    pub fn remove_file(&mut self, slug: &Slug, path: &str) -> Result<()> {
        let block = self.block_mut(slug)?;
        let paths = block.paths();
        let idx = block
            .files
            .iter()
            .position(|f| f.path == path)
            .ok_or_else(|| Error::PathNotInBlock {
                slug: slug.to_string(),
                path: path.to_string(),
                existing: paths,
            })?;
        let file = block.files.remove(idx);

        // Same rule as removing the block, and as a file leaving the review
        // window on its own: the prose outlives the arrangement it sat in.
        // Dropping it here and keeping it there would make which command you
        // typed decide whether your notes survive.
        let block_slug = slug.clone();
        for note in file.line_notes {
            self.orphans.push(Orphan {
                block: block_slug.clone(),
                path: file.path.clone(),
                old_range: note.range,
                snapshot: String::new(),
                reason: OrphanReason::FileRemoved,
                text: note.text,
            });
        }
        Ok(())
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
    fn files_keep_insertion_order_and_honour_after() {
        let mut m = map_with(&["one"]);
        m.add_file(&slug("one"), "a.rs", None, None).unwrap();
        m.add_file(&slug("one"), "c.rs", None, None).unwrap();
        m.add_file(&slug("one"), "b.rs", None, Some("a.rs"))
            .unwrap();
        let paths: Vec<_> = m
            .block(&slug("one"))
            .unwrap()
            .files
            .iter()
            .map(|f| &f.path)
            .collect();
        assert_eq!(paths, vec!["a.rs", "b.rs", "c.rs"]);
    }

    #[test]
    fn the_same_file_twice_in_one_block_is_rejected() {
        let mut m = map_with(&["one"]);
        m.add_file(&slug("one"), "a.rs", None, None).unwrap();
        let err = m.add_file(&slug("one"), "a.rs", None, None).unwrap_err();
        assert!(matches!(err, Error::DuplicatePath { .. }));
    }
}
