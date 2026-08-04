//! The map's self-test.
//!
//! It exists so "every file shows up somewhere" does not depend on the model
//! remembering the rule. A file nobody assigned is not merely undocumented on
//! screen — it is invisible, because the sidebar is built from the map.

use crate::diff::domain::DiffSource;
use crate::map::domain::ReviewMap;
use crate::shared::error::Result;

#[derive(Debug, Default)]
pub struct CheckReport {
    pub uncovered: Vec<String>,
    pub pending_orphans: usize,
    pub commits_behind: u32,
}

impl CheckReport {
    pub fn passed(&self) -> bool {
        self.uncovered.is_empty() && self.pending_orphans == 0
    }
}

pub fn check(map: &ReviewMap, source: &dyn DiffSource) -> Result<CheckReport> {
    let scope = source.scope()?;
    let covered = map.covered_paths();

    let uncovered: Vec<String> = scope
        .files
        .iter()
        .map(|f| f.path.clone())
        .filter(|p| !covered.contains(p))
        .collect();

    let commits_behind = source.commits_ahead_of(&map.generated_at).unwrap_or(0);

    Ok(CheckReport {
        uncovered,
        pending_orphans: map.orphans.len(),
        commits_behind,
    })
}
