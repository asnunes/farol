//! Git, and the text differ, behind the ports.
//!
//! Split by what each part changes for: `git` talks to the repository, `window`
//! decides what is under review, `text_diff` turns two versions of a file into
//! hunks, and `gix_source` is the adapter that presents all three as ports.

mod git;
mod gix_source;
mod text_diff;
mod window;

pub use gix_source::*;
