//! The map itself: what the session that wrote the change knew about it.
//!
//! The struct and the questions asked of it live here; changing it is split by
//! what is being changed — blocks, the files inside them, the notes pinned to
//! lines, the files in focus, and the notes left over when code
//! moved.

mod blocks;
mod files;
mod line_notes;
mod orphans;

pub use orphans::NoteFate;

use serde::{Deserialize, Serialize};

use super::block::{Block, BlockFile, LineNote};
use super::error::{MapError, Result};
use super::orphan::{Orphan, OrphanReason};
use super::position::Position;
use super::range::LineRange;
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
    /// Private, both: every rule the map enforces — one block per slug, a
    /// file listed once, a note replaced rather than duplicated, prose kept when
    /// its file goes — lives in the methods that change them. A caller holding
    /// the `Vec` could sidestep every one.
    #[serde(default)]
    blocks: Vec<Block>,
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
            orphans: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// The blocks in reading order. Read-only: changing them goes through the
    /// methods that keep the order and the slugs consistent.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
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

    /// Every path one block reads.
    pub fn paths_in(&self, slug: &Slug) -> impl Iterator<Item = &str> {
        self.block(slug)
            .into_iter()
            .flat_map(|b| b.files.iter().map(|f| f.path.as_str()))
    }

    /// Every path the map accounts for, in any role.
    pub fn covered_paths(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .blocks
            .iter()
            .flat_map(|b| b.files.iter().map(|f| f.path.clone()))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The block a file is read in, if any block reads it.
    pub fn block_of(&self, path: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.file(path).is_some())
    }

    /// The block that claims a path.
    fn owner_of(&self, path: &str) -> Option<&Slug> {
        self.block_of(path).map(|b| &b.slug)
    }

    /// Every path in one block. The methods that change the map keep this
    /// true; this is what a map read from disk is held to.
    fn one_home_per_path(&self) -> std::result::Result<(), MapError> {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        let homes = self.blocks.iter().flat_map(|b| {
            b.files
                .iter()
                .map(move |f| (f.path.as_str(), b.slug.as_str()))
        });
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
    fn covered_paths_are_the_blocks_files_without_duplicates() {
        let mut m = map_with(&["one", "two"]);
        m.add_file(&slug("two"), "b.rs", None, None).unwrap();
        m.add_file(&slug("one"), "a.rs", None, None).unwrap();

        assert_eq!(
            m.covered_paths(),
            vec!["a.rs", "b.rs"],
            "sorted, so two callers cannot disagree about the order"
        );
    }

    #[test]
    fn a_stored_map_from_before_skim_was_dropped_still_reads() {
        // Its skim list is ignored, not migrated: those files come back as
        // unassigned, and the skill places them on the next map update.
        let json = serde_json::json!({
            "version": MAP_VERSION, "branch": "b", "base": "main", "generated_at": "h",
            "blocks": [], "skim": [{"path": "Cargo.lock", "reason": "generated"}],
        });

        let map = serde_json::from_value::<ReviewMap>(json).unwrap();

        assert!(map.covered_paths().is_empty());
    }

    #[test]
    fn a_fresh_map_carries_nothing_but_where_it_came_from() {
        let m = ReviewMap::new("feature/x", "main", "abc123");

        assert!(m.is_empty());
        assert_eq!(m.generated_at, "abc123");
        assert_eq!(m.version, MAP_VERSION);
        assert_eq!(m.parent, None);
    }
}
