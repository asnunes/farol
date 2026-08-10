//! Notes pinned to a span of lines inside a file.
use super::*;

impl ReviewMap {
    pub fn add_line_note(
        &mut self,
        slug: &Slug,
        path: &str,
        range: LineRange,
        text: impl Into<String>,
    ) -> Result<()> {
        let file = self.block_file_mut(slug, path)?;
        file.line_notes.retain(|n| n.range != range);
        file.line_notes.push(LineNote {
            range,
            text: text.into(),
        });
        file.line_notes.sort_by_key(|n| n.range);
        Ok(())
    }

    pub fn update_line_note(
        &mut self,
        slug: &Slug,
        path: &str,
        range: LineRange,
        text: impl Into<String>,
    ) -> Result<()> {
        let file = self.block_file_mut(slug, path)?;
        let note = file
            .line_notes
            .iter_mut()
            .find(|n| n.range == range)
            .ok_or_else(|| Error::NoSuchLineNote {
                slug: slug.to_string(),
                path: path.to_string(),
                range,
            })?;
        note.text = text.into();
        Ok(())
    }

    pub fn remove_line_note(&mut self, slug: &Slug, path: &str, range: LineRange) -> Result<()> {
        let file = self.block_file_mut(slug, path)?;
        let before = file.line_notes.len();
        file.line_notes.retain(|n| n.range != range);
        if file.line_notes.len() == before {
            return Err(Error::NoSuchLineNote {
                slug: slug.to_string(),
                path: path.to_string(),
                range,
            });
        }
        Ok(())
    }

    fn block_file_mut(&mut self, slug: &Slug, path: &str) -> Result<&mut BlockFile> {
        let block = self.block_mut(slug)?;
        let paths = block.paths();
        block.file_mut(path).ok_or_else(|| Error::PathNotInBlock {
            slug: slug.to_string(),
            path: path.to_string(),
            existing: paths,
        })
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
    fn line_notes_stay_sorted_and_re_adding_replaces() {
        let mut m = map_with(&["one"]);
        m.add_file(&slug("one"), "a.rs", None, None).unwrap();
        m.add_line_note(
            &slug("one"),
            "a.rs",
            LineRange::new(40, 50).unwrap(),
            "second",
        )
        .unwrap();
        m.add_line_note(
            &slug("one"),
            "a.rs",
            LineRange::new(10, 20).unwrap(),
            "first",
        )
        .unwrap();
        m.add_line_note(
            &slug("one"),
            "a.rs",
            LineRange::new(40, 50).unwrap(),
            "replaced",
        )
        .unwrap();
        let notes = &m
            .block(&slug("one"))
            .unwrap()
            .file("a.rs")
            .unwrap()
            .line_notes;
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].text, "first");
        assert_eq!(notes[1].text, "replaced");
    }

    #[test]
    fn a_note_on_a_file_the_block_does_not_hold_is_refused() {
        // The note would be written nowhere the screen ever looks.
        let mut m = ReviewMap::new("feature/x", "main", "abc123");
        m.add_block(&slug("core"), "t", "c", Position::End).unwrap();
        m.add_file(&slug("core"), "a.rs", None, None).unwrap();

        let err = m
            .add_line_note(
                &slug("core"),
                "elsewhere.rs",
                crate::testing::range(1, 2),
                "n",
            )
            .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("elsewhere.rs"), "{msg}");
        assert!(msg.contains("a.rs"), "{msg}");
    }
}
