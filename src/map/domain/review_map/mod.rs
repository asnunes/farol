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
use super::error::{MapError, Result};
use super::orphan::{Orphan, OrphanReason};
use super::position::Position;
use super::range::LineRange;
use super::skim_entry::SkimEntry;
use super::slug::Slug;

pub const MAP_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "StoredMap")]
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
            .ok_or_else(|| MapError::UnknownBlock {
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

    /// Every path one block accounts for, read files and skim alike.
    ///
    /// The two lists are separate on the map because they are read differently;
    /// to anything asking whether the block still holds anything they are one
    /// list, and joining them at each call site is how the two drift.
    pub fn paths_in(&self, slug: &Slug) -> impl Iterator<Item = &str> {
        self.block(slug)
            .into_iter()
            .flat_map(|b| b.files.iter().map(|f| f.path.as_str()))
            .chain(self.skim_for(slug).map(|s| s.path.as_str()))
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

    /// The block a file is read in, if any block reads it.
    pub fn block_of(&self, path: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.file(path).is_some())
    }

    /// The block that claims a path, as a file or as skim attached to it.
    fn owner_of(&self, path: &str) -> Option<&Slug> {
        self.block_of(path).map(|b| &b.slug).or_else(|| {
            self.skim
                .iter()
                .find(|s| s.path == path)
                .and_then(|s| s.block.as_ref())
        })
    }

    /// Fold a file listed in several blocks into the earliest, which is where
    /// the screen always read it. Maps written before a file belonged to one
    /// block still load: the later blocks' notes move to the file that stays,
    /// because the prose outlives the arrangement it sat in.
    fn one_block_per_file(mut self) -> Self {
        let mut home: std::collections::HashMap<String, (usize, usize)> = Default::default();
        let mut strays = Vec::new();
        for b in 0..self.blocks.len() {
            for file in std::mem::take(&mut self.blocks[b].files) {
                if home.contains_key(&file.path) {
                    strays.push(file);
                    continue;
                }
                home.insert(file.path.clone(), (b, self.blocks[b].files.len()));
                self.blocks[b].files.push(file);
            }
        }
        for stray in strays {
            let (b, f) = home[&stray.path];
            let file = &mut self.blocks[b].files[f];
            file.note = match (file.note.take(), stray.note) {
                (Some(kept), Some(moved)) => Some(format!("{kept}\n\n{moved}")),
                (kept, moved) => kept.or(moved),
            };
            file.line_notes.extend(stray.line_notes);
        }
        // Skim that named a block other than the one reading the file follows
        // the file, so the file still has one home.
        for entry in &mut self.skim {
            if let Some(&(b, _)) = home.get(&entry.path) {
                entry.block = Some(self.blocks[b].slug.clone());
            }
        }
        self
    }
}

/// The map as it is read from disk, before it is held to one block per file.
#[derive(Deserialize)]
struct StoredMap {
    version: u32,
    branch: String,
    base: String,
    generated_at: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    blocks: Vec<Block>,
    #[serde(default)]
    skim: Vec<SkimEntry>,
    #[serde(default)]
    orphans: Vec<Orphan>,
}

impl From<StoredMap> for ReviewMap {
    fn from(stored: StoredMap) -> Self {
        ReviewMap {
            version: stored.version,
            branch: stored.branch,
            base: stored.base,
            generated_at: stored.generated_at,
            parent: stored.parent,
            blocks: stored.blocks,
            skim: stored.skim,
            orphans: stored.orphans,
        }
        .one_block_per_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{map_with, slug};

    #[test]
    fn a_file_already_in_one_block_is_refused_in_another_naming_where_it_is() {
        let mut m = map_with(&["first", "second"]);
        m.add_file(&slug("first"), "shared.rs", None, None).unwrap();

        let err = m
            .add_file(&slug("second"), "shared.rs", None, None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("already in block 'first'"), "{err}");
        assert!(
            err.contains("farol file remove first shared.rs"),
            "the refusal carries the way to move it: {err}"
        );
    }

    #[test]
    fn skim_may_only_name_the_block_that_reads_the_file() {
        let mut m = map_with(&["first", "second"]);
        m.add_file(&slug("first"), "a.rs", None, None).unwrap();

        assert!(
            m.add_skim("a.rs", "rename only", Some(slug("second")))
                .is_err()
        );
        assert!(
            m.add_skim("a.rs", "rename only", Some(slug("first")))
                .is_ok()
        );
    }

    #[test]
    fn a_map_written_with_a_file_in_two_blocks_keeps_it_in_the_first_with_every_note() {
        // Maps written before a file belonged to one block still load, and
        // nothing the session wrote about the file is lost on the way.
        let json = serde_json::json!({
            "version": MAP_VERSION, "branch": "b", "base": "main", "generated_at": "h",
            "blocks": [
                {"slug": "first", "title": "t", "context": "c",
                 "files": [{"path": "shared.rs", "note": "why it changed first"}]},
                {"slug": "second", "title": "t", "context": "c",
                 "files": [{"path": "shared.rs", "note": "why it changed again",
                            "line_notes": [{"from": 3, "to": 4, "text": "this part"}]}]},
            ],
            "skim": [{"path": "shared.rs", "reason": "r", "block": "second"}],
        });

        let m: ReviewMap = serde_json::from_value(json).unwrap();

        assert!(m.block(&slug("second")).unwrap().files.is_empty());
        let kept = m.block(&slug("first")).unwrap().file("shared.rs").unwrap();
        assert_eq!(
            kept.note.as_deref(),
            Some("why it changed first\n\nwhy it changed again")
        );
        assert_eq!(kept.line_notes.len(), 1);
        assert_eq!(
            m.skim()[0].block,
            Some(slug("first")),
            "skim follows the file"
        );
    }

    #[test]
    fn covered_paths_span_blocks_and_skim_without_duplicates() {
        let mut m = map_with(&["one", "two"]);
        m.add_file(&slug("one"), "a.rs", None, None).unwrap();
        m.add_file(&slug("two"), "b.rs", None, None).unwrap();
        m.add_skim("a.rs", "rename only", Some(slug("one")))
            .unwrap();
        m.add_skim("Cargo.lock", "generated", None).unwrap();

        assert_eq!(
            m.covered_paths(),
            vec!["Cargo.lock", "a.rs", "b.rs"],
            "sorted, so two callers cannot disagree about the order"
        );
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
        m.add_skim("Cargo.lock", "generated", None).unwrap();

        assert!(!m.is_empty());
    }
}
