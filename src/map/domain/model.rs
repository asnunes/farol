use serde::{Deserialize, Serialize};

use super::range::LineRange;

pub use crate::shared::WORKING;
use crate::shared::error::{Error, Result};

pub const MAP_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrphanReason {
    /// The code under the note was rewritten.
    HunkOverlap,
    FileRenamed,
    FileRemoved,
    BlockRemoved,
}

impl OrphanReason {
    pub fn label(&self) -> &'static str {
        match self {
            OrphanReason::HunkOverlap => "hunk-overlap",
            OrphanReason::FileRenamed => "file-renamed",
            OrphanReason::FileRemoved => "file-removed",
            OrphanReason::BlockRemoved => "block-removed",
        }
    }

    pub fn guidance(&self) -> &'static str {
        match self {
            OrphanReason::HunkOverlap => {
                "re-read the new code; restore with the new range if the note still holds, otherwise discard"
            }
            OrphanReason::FileRenamed => "restore against the new path",
            OrphanReason::FileRemoved => "discard — there is nowhere to restore it",
            OrphanReason::BlockRemoved => "discard, or restore into another block",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineNote {
    #[serde(flatten)]
    pub range: LineRange,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockFile {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_notes: Vec<LineNote>,
}

impl BlockFile {
    pub fn new(path: impl Into<String>, note: Option<String>) -> Self {
        Self {
            path: path.into(),
            note,
            line_notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub slug: String,
    pub title: String,
    pub context: String,
    #[serde(default)]
    pub files: Vec<BlockFile>,
}

impl Block {
    pub fn file(&self, path: &str) -> Option<&BlockFile> {
        self.files.iter().find(|f| f.path == path)
    }

    pub fn file_mut(&mut self, path: &str) -> Option<&mut BlockFile> {
        self.files.iter_mut().find(|f| f.path == path)
    }

    fn paths(&self) -> Vec<String> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }
}

/// A file the reviewer may read diagonally. `block` is optional on purpose:
/// a test fixture that only changed because of block 3 belongs next to block 3,
/// but a lockfile belongs to no story at all, and forcing one would be the same
/// mistake as inventing a block to hold leftovers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkimEntry {
    pub path: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<String>,
}

/// A line note whose anchor stopped being trustworthy. The prose is kept — it
/// was the expensive part — along with the code it used to cover, which is what
/// actually identifies it. The old range is only a hint about where to look.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Orphan {
    pub block: String,
    pub path: String,
    #[serde(flatten)]
    pub old_range: LineRange,
    pub snapshot: String,
    pub reason: OrphanReason,
    pub text: String,
}

/// Where a newly added block or file goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    End,
    Before(String),
    After(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewMap {
    pub version: u32,
    pub branch: String,
    pub base: String,
    /// Commit this version was built against, or `working` for uncommitted work.
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub skim: Vec<SkimEntry>,
    /// Deactivated notes waiting for a decision. Lives only in the version that
    /// produced it — the next derivation does not inherit it.
    #[serde(default)]
    pub orphans: Vec<Orphan>,
}

impl ReviewMap {
    pub fn new(
        branch: impl Into<String>,
        base: impl Into<String>,
        generated_at: impl Into<String>,
    ) -> Self {
        Self {
            version: MAP_VERSION,
            branch: branch.into(),
            base: base.into(),
            generated_at: generated_at.into(),
            parent: None,
            blocks: Vec::new(),
            skim: Vec::new(),
            orphans: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty() && self.skim.is_empty()
    }

    pub fn slugs(&self) -> Vec<String> {
        self.blocks.iter().map(|b| b.slug.clone()).collect()
    }

    pub fn block(&self, slug: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.slug == slug)
    }

    pub fn index_of(&self, slug: &str) -> Option<usize> {
        self.blocks.iter().position(|b| b.slug == slug)
    }

    fn block_mut(&mut self, slug: &str) -> Result<&mut Block> {
        let existing = self.slugs();
        self.blocks
            .iter_mut()
            .find(|b| b.slug == slug)
            .ok_or_else(|| Error::UnknownBlock {
                slug: slug.to_string(),
                existing,
            })
    }

    /// Every path the map accounts for, in any role.
    pub fn covered_paths(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .blocks
            .iter()
            .flat_map(|b| b.files.iter().map(|f| f.path.clone()))
            .collect();
        out.extend(self.skim.iter().map(|s| s.path.clone()));
        out.sort();
        out.dedup();
        out
    }

    /// The block a file is read in: the earliest one that holds it. Ordering
    /// decides this, so there is nothing to choose.
    pub fn first_block_of(&self, path: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.file(path).is_some())
    }

    /// Every block a file participates in, in reading order. The screen turns
    /// this into the tag list next to the path.
    pub fn blocks_of(&self, path: &str) -> Vec<&Block> {
        self.blocks
            .iter()
            .filter(|b| b.file(path).is_some())
            .collect()
    }

    // ---- blocks -------------------------------------------------------

    pub fn add_block(
        &mut self,
        slug: impl Into<String>,
        title: impl Into<String>,
        context: impl Into<String>,
        position: Position,
    ) -> Result<()> {
        let slug = slug.into();
        if self.index_of(&slug).is_some() {
            return Err(Error::DuplicateBlock { slug });
        }
        let at = self.resolve_position(&position)?;
        self.blocks.insert(
            at,
            Block {
                slug,
                title: title.into(),
                context: context.into(),
                files: Vec::new(),
            },
        );
        Ok(())
    }

    pub fn update_block(
        &mut self,
        slug: &str,
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
    pub fn remove_block(&mut self, slug: &str) -> Result<()> {
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
            if entry.block.as_deref() == Some(slug) {
                entry.block = None;
            }
        }
        Ok(())
    }

    pub fn move_block(&mut self, slug: &str, position: Position) -> Result<()> {
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
                slug: target.clone(),
                existing: self.slugs(),
            }),
            Position::After(target) => {
                self.index_of(target)
                    .map(|i| i + 1)
                    .ok_or_else(|| Error::UnknownBlock {
                        slug: target.clone(),
                        existing: self.slugs(),
                    })
            }
        }
    }

    // ---- files --------------------------------------------------------

    pub fn add_file(
        &mut self,
        slug: &str,
        path: impl Into<String>,
        note: Option<String>,
        after: Option<&str>,
    ) -> Result<()> {
        let path = path.into();
        let existing = self.slugs();
        let block = self
            .blocks
            .iter_mut()
            .find(|b| b.slug == slug)
            .ok_or_else(|| Error::UnknownBlock {
                slug: slug.to_string(),
                existing,
            })?;

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

    pub fn update_file(&mut self, slug: &str, path: &str, note: Option<String>) -> Result<()> {
        let existing = self.slugs();
        let block = self
            .blocks
            .iter_mut()
            .find(|b| b.slug == slug)
            .ok_or_else(|| Error::UnknownBlock {
                slug: slug.to_string(),
                existing,
            })?;
        let paths = block.paths();
        let file = block.file_mut(path).ok_or_else(|| Error::PathNotInBlock {
            slug: slug.to_string(),
            path: path.to_string(),
            existing: paths,
        })?;
        file.note = note;
        Ok(())
    }

    pub fn remove_file(&mut self, slug: &str, path: &str) -> Result<()> {
        let existing = self.slugs();
        let block = self
            .blocks
            .iter_mut()
            .find(|b| b.slug == slug)
            .ok_or_else(|| Error::UnknownBlock {
                slug: slug.to_string(),
                existing,
            })?;
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
        block.files.remove(idx);
        Ok(())
    }

    // ---- line notes ---------------------------------------------------

    pub fn add_line_note(
        &mut self,
        slug: &str,
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
        slug: &str,
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

    pub fn remove_line_note(&mut self, slug: &str, path: &str, range: LineRange) -> Result<()> {
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

    fn block_file_mut(&mut self, slug: &str, path: &str) -> Result<&mut BlockFile> {
        let existing = self.slugs();
        let block = self
            .blocks
            .iter_mut()
            .find(|b| b.slug == slug)
            .ok_or_else(|| Error::UnknownBlock {
                slug: slug.to_string(),
                existing,
            })?;
        let paths = block.paths();
        block.file_mut(path).ok_or_else(|| Error::PathNotInBlock {
            slug: slug.to_string(),
            path: path.to_string(),
            existing: paths,
        })
    }

    // ---- orphans ------------------------------------------------------

    pub fn take_orphan(&mut self, slug: &str, path: &str, range: LineRange) -> Result<Orphan> {
        let idx = self
            .orphans
            .iter()
            .position(|o| o.block == slug && o.path == path && o.old_range == range)
            .ok_or_else(|| Error::NoSuchOrphan {
                slug: slug.to_string(),
                path: path.to_string(),
                range,
            })?;
        Ok(self.orphans.remove(idx))
    }

    // ---- skim ---------------------------------------------------------

    pub fn add_skim(
        &mut self,
        path: impl Into<String>,
        reason: impl Into<String>,
        block: Option<String>,
    ) -> Result<()> {
        let path = path.into();
        if let Some(slug) = &block
            && self.index_of(slug).is_none()
        {
            return Err(Error::UnknownBlock {
                slug: slug.clone(),
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
            return Err(Error::msg(format!("'{path}' is not marked as skim")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with(slugs: &[&str]) -> ReviewMap {
        let mut m = ReviewMap::new("feature/x", "main", "abc123");
        for s in slugs {
            m.add_block(*s, "t", "c", Position::End).unwrap();
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
        let err = m.add_block("one", "t", "c", Position::End).unwrap_err();
        assert!(matches!(err, Error::DuplicateBlock { .. }));
    }

    #[test]
    fn add_before_lands_ahead_of_the_target() {
        let mut m = map_with(&["one", "two"]);
        m.add_block("mid", "t", "c", Position::Before("two".into()))
            .unwrap();
        assert_eq!(m.slugs(), vec!["one", "mid", "two"]);
    }

    #[test]
    fn add_after_lands_behind_the_target() {
        let mut m = map_with(&["one", "two"]);
        m.add_block("mid", "t", "c", Position::After("one".into()))
            .unwrap();
        assert_eq!(m.slugs(), vec!["one", "mid", "two"]);
    }

    #[test]
    fn add_before_the_first_block_reaches_the_top() {
        let mut m = map_with(&["one", "two"]);
        m.add_block("zero", "t", "c", Position::Before("one".into()))
            .unwrap();
        assert_eq!(m.slugs(), vec!["zero", "one", "two"]);
    }

    #[test]
    fn positioning_against_an_unknown_slug_fails_and_lists_the_real_ones() {
        let mut m = map_with(&["one", "two"]);
        let err = m
            .add_block("x", "t", "c", Position::After("nope".into()))
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
        m.move_block("three", Position::Before("one".into()))
            .unwrap();
        assert_eq!(m.slugs(), vec!["three", "one", "two"]);
        m.move_block("three", Position::End).unwrap();
        assert_eq!(m.slugs(), vec!["one", "two", "three"]);
    }

    #[test]
    fn failed_move_leaves_the_block_where_it_was() {
        let mut m = map_with(&["one", "two"]);
        let err = m
            .move_block("one", Position::After("ghost".into()))
            .unwrap_err();
        assert!(matches!(err, Error::UnknownBlock { .. }));
        assert_eq!(m.slugs(), vec!["one", "two"]);
    }

    #[test]
    fn files_keep_insertion_order_and_honour_after() {
        let mut m = map_with(&["one"]);
        m.add_file("one", "a.rs", None, None).unwrap();
        m.add_file("one", "c.rs", None, None).unwrap();
        m.add_file("one", "b.rs", None, Some("a.rs")).unwrap();
        let paths: Vec<_> = m
            .block("one")
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
        m.add_file("one", "a.rs", None, None).unwrap();
        let err = m.add_file("one", "a.rs", None, None).unwrap_err();
        assert!(matches!(err, Error::DuplicatePath { .. }));
    }

    #[test]
    fn a_file_in_two_blocks_reads_in_the_earlier_one() {
        let mut m = map_with(&["first", "second"]);
        m.add_file("second", "shared.rs", None, None).unwrap();
        m.add_file("first", "shared.rs", None, None).unwrap();
        assert_eq!(m.first_block_of("shared.rs").unwrap().slug, "first");
        let tags: Vec<_> = m
            .blocks_of("shared.rs")
            .iter()
            .map(|b| b.slug.clone())
            .collect();
        assert_eq!(tags, vec!["first", "second"]);
    }

    #[test]
    fn line_notes_stay_sorted_and_re_adding_replaces() {
        let mut m = map_with(&["one"]);
        m.add_file("one", "a.rs", None, None).unwrap();
        m.add_line_note("one", "a.rs", LineRange::new(40, 50).unwrap(), "second")
            .unwrap();
        m.add_line_note("one", "a.rs", LineRange::new(10, 20).unwrap(), "first")
            .unwrap();
        m.add_line_note("one", "a.rs", LineRange::new(40, 50).unwrap(), "replaced")
            .unwrap();
        let notes = &m.block("one").unwrap().file("a.rs").unwrap().line_notes;
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].text, "first");
        assert_eq!(notes[1].text, "replaced");
    }

    #[test]
    fn removing_a_block_keeps_its_line_notes_as_orphans() {
        let mut m = map_with(&["one"]);
        m.add_file("one", "a.rs", None, None).unwrap();
        m.add_line_note(
            "one",
            "a.rs",
            LineRange::new(10, 20).unwrap(),
            "worth keeping",
        )
        .unwrap();
        m.remove_block("one").unwrap();
        assert_eq!(m.orphans.len(), 1);
        assert_eq!(m.orphans[0].reason, OrphanReason::BlockRemoved);
        assert_eq!(m.orphans[0].text, "worth keeping");
    }

    #[test]
    fn removing_a_block_detaches_skim_entries_that_pointed_at_it() {
        let mut m = map_with(&["one"]);
        m.add_skim("go.sum", "generated", Some("one".into()))
            .unwrap();
        m.remove_block("one").unwrap();
        assert_eq!(m.skim[0].block, None);
    }

    #[test]
    fn skim_cannot_point_at_a_block_that_does_not_exist() {
        let mut m = map_with(&["one"]);
        let err = m
            .add_skim("go.sum", "generated", Some("ghost".into()))
            .unwrap_err();
        assert!(matches!(err, Error::UnknownBlock { .. }));
    }

    #[test]
    fn covered_paths_span_blocks_and_skim_without_duplicates() {
        let mut m = map_with(&["one", "two"]);
        m.add_file("one", "a.rs", None, None).unwrap();
        m.add_file("two", "a.rs", None, None).unwrap();
        m.add_file("two", "b.rs", None, None).unwrap();
        m.add_skim("go.sum", "generated", None).unwrap();
        assert_eq!(m.covered_paths(), vec!["a.rs", "b.rs", "go.sum"]);
    }
}
