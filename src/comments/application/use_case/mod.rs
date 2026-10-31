//! What the routes and the commands call on threads and pending comments.
//!
//! One struct, one `execute`, one file — the same shape the map's use cases
//! have, and for the same reason: the name of the operation is the name of the
//! thing you open.

mod choose_mode;
mod close_thread;
mod edit_pending;
mod generate_pending;
mod generate_pull_draft;
mod open_pull_request;
mod publish_review;
mod remove_pending;
mod review_readiness;
mod send_comment;
mod sync_read_marks;
mod write_pending;
mod write_pull_draft;

pub use choose_mode::*;
pub use close_thread::*;
pub use edit_pending::*;
pub use generate_pending::*;
pub use generate_pull_draft::*;
pub use open_pull_request::*;
pub use publish_review::*;
pub use remove_pending::*;
pub use review_readiness::*;
pub use send_comment::*;
pub use sync_read_marks::*;
pub use write_pending::*;
pub use write_pull_draft::*;
