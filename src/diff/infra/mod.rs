//! Git, and the text differ, behind the ports.
//!
//! Split by what each part changes for: `blob` names a file's contents the way
//! git does, `git` talks to the repository, `window` decides what is under
//! review, `text_diff` turns two versions of a file into hunks, and
//! `gix_source` is the adapter that presents them as ports.

mod blob;
#[cfg(test)]
mod fixture;
mod git;
mod gix_head;
mod gix_source;
mod text_diff;
mod window;

pub use gix_head::GixHead;
pub use gix_source::*;
