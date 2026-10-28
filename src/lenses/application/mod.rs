//! What the CLI and the server call about lenses, and the reading of them the
//! screen is drawn from.

mod read_files;
mod reading;
mod use_case;

pub use read_files::*;
pub use reading::*;
pub use use_case::*;

use crate::diff::domain::{ReviewPath, Scope};
use crate::lenses::domain::{Covered, LensTarget};
use crate::map::domain::{ReviewMap, Slug};

/// Where an answer is being written, with the block or file already proven.
#[derive(Debug, Clone)]
pub enum Aim {
    Pr,
    Block(Slug),
    File(ReviewPath),
}

/// The files a target covers as the review stands: every file for the pull
/// request, a block's own, or the one file. Answers remember this set, and
/// the set moving is how an answer becomes Outdated.
pub(crate) fn covered_now(target: &LensTarget, map: &ReviewMap, scope: &Scope) -> Vec<Covered> {
    let in_scope = |path: &str| {
        scope.change(path).map(|change| Covered {
            path: change.path.clone(),
            blob: change.content_hash.clone(),
        })
    };
    let mut covered: Vec<Covered> = match target {
        LensTarget::Pr => scope
            .files
            .iter()
            .filter_map(|f| in_scope(&f.path))
            .collect(),
        LensTarget::Block { slug } => map.paths_in(slug).filter_map(in_scope).collect(),
        LensTarget::File { path } => in_scope(path).into_iter().collect(),
    };
    covered.sort();
    covered.dedup();
    covered
}
