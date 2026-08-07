//! The map itself: what the session that wrote the change knew about it.
//!
//! The struct and the questions asked of it live here; changing it is split by
//! what is being changed — blocks, the files inside them, the notes pinned to
//! lines, the skim list, and the notes left over when code moved.

mod blocks;
mod files;
mod line_notes;
mod orphans;
mod skim;

pub use orphans::NoteFate;

use serde::{Deserialize, Serialize};

use super::block::{Block, BlockFile, LineNote};
use super::orphan::{Orphan, OrphanReason};
use super::position::Position;
use super::range::LineRange;
use super::skim_entry::SkimEntry;
use super::slug::Slug;
use crate::shared::error::{Error, Result};

pub const MAP_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewMap {
    pub version: u32,
    pub branch: String,
    pub base: String,
    /// Commit this version was built against, or `working` for uncommitted work.
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Private, all three: every rule the map enforces — one block per slug, a
    /// file listed once, a note replaced rather than duplicated, prose kept when
    /// its file goes — lives in the methods that change them. A caller holding
    /// the `Vec` could sidestep every one.
    #[serde(default)]
    blocks: Vec<Block>,
    #[serde(default)]
    skim: Vec<SkimEntry>,
    /// Deactivated notes waiting for a decision. Lives only in the version that
    /// produced it — the next derivation does not inherit it.
    #[serde(default)]
    orphans: Vec<Orphan>,
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
        self.blocks.is_empty() && self.skim().is_empty()
    }

    /// The blocks in reading order. Read-only: changing them goes through the
    /// methods that keep the order and the slugs consistent.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn skim(&self) -> &[SkimEntry] {
        &self.skim
    }

    pub fn slugs(&self) -> Vec<String> {
        self.blocks.iter().map(|b| b.slug.to_string()).collect()
    }

    pub fn block(&self, slug: &Slug) -> Option<&Block> {
        self.blocks.iter().find(|b| &b.slug == slug)
    }

    pub fn index_of(&self, slug: &Slug) -> Option<usize> {
        self.blocks.iter().position(|b| &b.slug == slug)
    }

    fn block_mut(&mut self, slug: &Slug) -> Result<&mut Block> {
        let existing = self.slugs();
        self.blocks
            .iter_mut()
            .find(|b| &b.slug == slug)
            .ok_or_else(|| Error::UnknownBlock {
                slug: slug.to_string(),
                existing,
            })
    }

    /// Skim entries attached to a block. Both the renderer and the screen need
    /// this split, and having each decide it separately is how the two drift.
    pub fn skim_for(&self, slug: &Slug) -> impl Iterator<Item = &SkimEntry> {
        self.skim
            .iter()
            .filter(move |s| s.block.as_ref() == Some(slug))
    }

    /// Skim entries belonging to no block — a lockfile has no story to sit in.
    pub fn loose_skim(&self) -> impl Iterator<Item = &SkimEntry> {
        self.skim().iter().filter(|s| s.block.is_none())
    }

    /// Every path the map accounts for, in any role.
    pub fn covered_paths(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .blocks
            .iter()
            .flat_map(|b| b.files.iter().map(|f| f.path.clone()))
            .collect();
        out.extend(self.skim().iter().map(|s| s.path.clone()));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::slug;

    fn map_with(slugs: &[&str]) -> ReviewMap {
        let mut map = ReviewMap::new("feature/x", "main", "abc123");
        for s in slugs {
            map.add_block(&slug(s), "t", "c", Position::End).unwrap();
        }
        map
    }

    #[test]
    fn a_file_in_two_blocks_reads_in_the_earlier_one() {
        let mut m = map_with(&["first", "second"]);
        m.add_file(&slug("second"), "shared.rs", None, None)
            .unwrap();
        m.add_file(&slug("first"), "shared.rs", None, None).unwrap();

        assert_eq!(
            m.first_block_of("shared.rs").unwrap().slug.as_str(),
            "first"
        );
        let tags: Vec<String> = m
            .blocks_of("shared.rs")
            .iter()
            .map(|b| b.slug.to_string())
            .collect();
        assert_eq!(tags, vec!["first", "second"]);
    }

    #[test]
    fn covered_paths_span_blocks_and_skim_without_duplicates() {
        let mut m = map_with(&["one", "two"]);
        m.add_file(&slug("one"), "a.rs", None, None).unwrap();
        m.add_file(&slug("two"), "a.rs", None, None).unwrap();
        m.add_file(&slug("two"), "b.rs", None, None).unwrap();
        m.add_skim("go.sum", "generated", None).unwrap();

        assert_eq!(m.covered_paths(), vec!["a.rs", "b.rs", "go.sum"]);
    }

    #[test]
    fn a_fresh_map_carries_nothing_but_where_it_came_from() {
        let m = ReviewMap::new("feature/x", "main", "abc123");

        assert!(m.is_empty());
        assert_eq!(m.generated_at, "abc123");
        assert_eq!(m.version, MAP_VERSION);
        assert_eq!(m.parent, None);
    }

    #[test]
    fn a_map_holding_only_skim_entries_is_not_empty() {
        // "Nothing to read closely" is still a decision the reviewer made.
        let mut m = ReviewMap::new("feature/x", "main", "abc123");
        m.add_skim("go.sum", "generated", None).unwrap();

        assert!(!m.is_empty());
    }
}
