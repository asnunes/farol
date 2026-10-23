//! What the page is sent, one file per subject: the review, a file's diff, the
//! threads, and the pull request.
//!
//! The rendering rules live here rather than in the browser, so the frontend
//! stays a renderer: a file is read in the earliest block that holds it, its
//! notes from *every* block travel with it, and the tags say which stories it
//! belongs to.

mod diff;
mod github;
mod review;
mod threads;

pub use diff::*;
pub use github::*;
pub use review::*;
pub use threads::*;
