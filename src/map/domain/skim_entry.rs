//! A file the reviewer may read diagonally.

use serde::{Deserialize, Serialize};

use super::slug::Slug;

/// A file the reviewer may read diagonally. `block` is optional on purpose:
/// a test fixture that only changed because of block 3 belongs next to block 3,
/// but a lockfile belongs to no story at all, and forcing one would be the same
/// mistake as inventing a block to hold leftovers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkimEntry {
    pub path: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<Slug>,
}
