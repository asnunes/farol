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
#[serde(try_from = "StoredMap")]
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

    /// Every path in one place: one block's files, or the skim list — skim
    /// attached to a block counting as that block. The methods that change the
    /// map keep this true; this is what a map read from disk is held to.
    fn one_home_per_path(&self) -> std::result::Result<(), MapError> {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        let homes = self
            .blocks
            .iter()
            .flat_map(|b| {
                b.files
                    .iter()
                    .map(move |f| (f.path.as_str(), b.slug.as_str()))
            })
            .chain(self.skim.iter().filter_map(|s| {
                // Skim in the block that reads the file is the same home.
                let home = s.block.as_ref().map(|b| b.as_str()).unwrap_or("skim");
                let reads = self.block_of(&s.path).map(|b| b.slug.as_str());
                (reads != Some(home)).then_some((s.path.as_str(), home))
            }));
        for (path, home) in homes {
            if let Some((_, first)) = seen.iter().find(|(p, _)| *p == path) {
                return Err(MapError::SharedPath {
                    path: path.to_string(),
                    first: first.to_string(),
                    second: home.to_string(),
                });
            }
            seen.push((path, home));
        }
        Ok(())
    }
}

/// The map as it is read from disk, before it is held to the map's rules.
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

/// Held to the map's rules on the way in, and refused when it breaks one:
/// a stored map is not migrated, it is discarded and the reader is told why.
impl TryFrom<StoredMap> for ReviewMap {
    type Error = MapError;

    fn try_from(stored: StoredMap) -> std::result::Result<Self, MapError> {
        let map = ReviewMap {
            version: stored.version,
            branch: stored.branch,
            base: stored.base,
            generated_at: stored.generated_at,
            parent: stored.parent,
            blocks: stored.blocks,
            skim: stored.skim,
            orphans: stored.orphans,
        };
        map.one_home_per_path()?;
        Ok(map)
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
    fn a_stored_map_with_a_file_in_two_blocks_is_refused_rather_than_rewritten() {
        // Stored state is not migrated: a map that breaks the rule is
        // discarded by whoever reads it, and the reason names the file.
        let json = serde_json::json!({
            "version": MAP_VERSION, "branch": "b", "base": "main", "generated_at": "h",
            "blocks": [
                {"slug": "first", "title": "t", "context": "c", "files": [{"path": "shared.rs"}]},
                {"slug": "second", "title": "t", "context": "c", "files": [{"path": "shared.rs"}]},
            ],
        });

        let err = serde_json::from_value::<ReviewMap>(json)
            .unwrap_err()
            .to_string();

        assert!(err.contains("shared.rs"), "{err}");
        assert!(err.contains("'first' and 'second'"), "{err}");
    }

    #[test]
    fn skim_in_the_block_that_reads_the_file_is_not_a_second_home() {
        let mut m = map_with(&["first"]);
        m.add_file(&slug("first"), "a.rs", None, None).unwrap();
        m.add_skim("a.rs", "rename only", None).unwrap();

        let back: ReviewMap = serde_json::from_value(serde_json::to_value(&m).unwrap()).unwrap();

        assert_eq!(back, m);
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
