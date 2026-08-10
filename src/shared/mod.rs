pub mod paths;

/// Stands in for a commit id when the map covers uncommitted work.
///
/// Both layers need it — the diff layer to know it should read the working
/// tree, the map layer to name the version — so it belongs to neither.
pub const WORKING: &str = "working";
