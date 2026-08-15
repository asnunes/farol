//! What the CLI and the HTTP handlers call.
//!
//! A use case is the first point of contact with business logic: a command or a
//! route invokes one and does nothing else with the domain. Services and
//! repositories are its dependencies, never the transport's.
//!
//! Each one is a struct holding what it needs and a single `execute`, in a file
//! of its own. That the struct exists at all is the point — it names the
//! operation, states its dependencies in its constructor, and can be built in a
//! test with fakes. One per file so the name of the operation is the name of
//! the thing you open, and so its tests have nowhere to drift to.

mod add_block;
mod add_file;
mod add_line_note;
mod add_skim;
mod check_map;
mod derive_map;
mod discard_note;
mod export_map;
mod get_file_diff;
mod get_review;
mod get_scope;
mod import_map;
mod move_block;
mod remove_block;
mod remove_file;
mod remove_line_note;
mod remove_skim;
mod reset_map;
mod restore_note;
mod show_map;
mod update_block;
mod update_file;
mod update_line_note;

pub use add_block::*;
pub use add_file::*;
pub use add_line_note::*;
pub use add_skim::*;
pub use check_map::*;
pub use derive_map::*;
pub use discard_note::*;
pub use export_map::*;
pub use get_file_diff::*;
pub use get_review::*;
pub use get_scope::*;
pub use import_map::*;
pub use move_block::*;
pub use remove_block::*;
pub use remove_file::*;
pub use remove_line_note::*;
pub use remove_skim::*;
pub use reset_map::*;
pub use restore_note::*;
pub use show_map::*;
pub use update_block::*;
pub use update_file::*;
pub use update_line_note::*;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::slug;

    #[test]
    fn before_wins_over_after_when_both_are_given() {
        assert_eq!(
            position_from(Some(slug("x")), Some(slug("y"))),
            Position::Before(slug("x"))
        );
    }

    #[test]
    fn after_alone_places_it_behind_the_named_block() {
        assert_eq!(
            position_from(None, Some(slug("y"))),
            Position::After(slug("y"))
        );
    }

    #[test]
    fn no_flags_means_append() {
        assert_eq!(position_from(None, None), Position::End);
    }
}
