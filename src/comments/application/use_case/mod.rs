//! What the routes and the commands call on threads and pending comments.
//!
//! One struct, one `execute`, one file — the same shape the map's use cases
//! have, and for the same reason: the name of the operation is the name of the
//! thing you open.

mod acknowledge_thread;
mod add_thread;
mod choose_mode;
mod close_thread;
mod edit_pending;
mod generate_pending;
mod generate_pull_draft;
mod list_threads;
mod open_pull_request;
mod publish_review;
mod push_branch;
mod remove_pending;
mod reply_to_thread;
mod review_readiness;
mod send_comment;
mod show_pull_draft;
mod sync_read_marks;
mod write_pending;
mod write_pull_draft;

pub use acknowledge_thread::*;
pub use add_thread::*;
pub use choose_mode::*;
pub use close_thread::*;
pub use edit_pending::*;
pub use generate_pending::*;
pub use generate_pull_draft::*;
pub use list_threads::*;
pub use open_pull_request::*;
pub use publish_review::*;
pub use push_branch::*;
pub use remove_pending::*;
pub use reply_to_thread::*;
pub use review_readiness::*;
pub use send_comment::*;
pub use show_pull_draft::*;
pub use sync_read_marks::*;
pub use write_pending::*;
pub use write_pull_draft::*;
