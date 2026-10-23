//! What the page is sent, one file per subject: the review, a file's diff, the
//! threads, and the pull request.
//!
//! The rendering rules live here rather than in the browser, so the frontend
//! stays a renderer.

mod diff;
mod github;
mod review;
mod threads;

pub use diff::*;
pub use github::*;
pub use review::*;
pub use threads::*;
