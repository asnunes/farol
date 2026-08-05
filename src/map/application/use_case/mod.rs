//! What the CLI and the HTTP handlers call.
//!
//! A use case is the first point of contact with business logic: a command or a
//! route invokes one and does nothing else with the domain. Services and
//! repositories are its dependencies, never the transport's.
//!
//! Each one is a struct holding what it needs and a single `execute`. That the
//! struct exists at all is the point — it names the operation, states its
//! dependencies in its constructor, and can be built in a test with fakes.

mod block;
mod file;
mod line;
mod review;
mod skim;

pub use block::*;
pub use file::*;
pub use line::*;
pub use review::*;
pub use skim::*;

use crate::map::domain::Position;
use crate::map::domain::Slug;

/// Where a newly added block or file goes, from the two CLI flags.
pub fn position_from(before: Option<Slug>, after: Option<Slug>) -> Position {
    match (before, after) {
        (Some(b), _) => Position::Before(b),
        (None, Some(a)) => Position::After(a),
        (None, None) => Position::End,
    }
}
