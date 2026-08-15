//! The shape a map takes to leave the machine it was written on.
//!
//! Both directions read it, so it lives beside the other services rather than
//! inside either use case.

use crate::map::domain::ReviewMap;

/// There is no version of its own: the map carries the only one there is, and a
/// second number beside it would be a second thing to keep in step. A file that
/// is not one of these fails to deserialise on a missing field, which is
/// refusal enough.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Bundle {
    /// Two labels, neither compared. They are here to be read by whoever opens
    /// the file wondering where it came from.
    pub repo: String,
    pub branch: String,
    /// What identifies the review: two clones that share a base are the same
    /// review, whatever their branches are called.
    pub base: String,
    /// Where the map was written. Reported when it differs from here, never
    /// refused — reading a review a few commits ahead of its map is ordinary.
    pub head: String,
    pub map: ReviewMap,
}
