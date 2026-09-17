//! What the routes call to publish.
//!
//! One struct, one `execute`, one file — the same shape the map's use cases
//! have, and for the same reason: the name of the operation is the name of the
//! thing you open.

mod publish_review;
mod review_readiness;

pub use publish_review::*;
pub use review_readiness::*;
