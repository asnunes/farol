use super::review_map::ReviewMap;
use crate::shared::error::Result;

/// Persistence for review maps, keyed by the commit each version was built
/// against. Swapping this is what it would take to make maps travel — commit
/// them, push them, export them — without touching anything else.
pub trait MapRepository: Send + Sync {
    /// Map stored for exactly this commit, if any.
    fn load_at(&self, sha: &str) -> Result<Option<ReviewMap>>;

    /// Commits that have a stored map, newest write first.
    fn stored_shas(&self) -> Result<Vec<String>>;

    fn save(&self, map: &ReviewMap) -> Result<()>;

    /// Drop the version stored for `sha`.
    fn delete(&self, sha: &str) -> Result<()>;
}
