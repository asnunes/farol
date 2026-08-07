//! Text for the one consumer these commands have: the skill.
//!
//! Not JSON. The model has to reason about the map — is this block still true,
//! does this note still hold — and prose buried in escaped strings fights that
//! for no gain. Labelled, indented text reads the way the model needs it.
//!
//! Each report borrows what it prints and implements `Display`, so callers hand
//! it straight to `print!` and nothing builds a `String` it does not need. One
//! report per file, for the same reason as one use case per file.

mod check_summary;
mod map_report;
mod orphan_report;
mod scope_report;
mod text;

pub use check_summary::*;
pub use map_report::*;
pub use orphan_report::*;
pub use scope_report::*;
