//! The lenses as the screen reads them: which are on, what each said, and
//! whether that still matches the review.

use crate::lenses::domain::{Freshness, LensAnswer, LensDefinition, LensGroup};
use crate::map::domain::Slug;

/// Every lens switched on, with its answers and how they stand.
#[derive(Debug, Clone, Default)]
pub struct LensReading {
    pub lenses: Vec<LensDefinition>,
    pub answers: Vec<(LensAnswer, Freshness)>,
}

impl LensReading {
    /// The pull request's lenses, in order, answered or not.
    pub fn for_pr(&self) -> Vec<Lensed<'_>> {
        self.in_group(LensGroup::Pr, |answer| {
            matches!(answer.target, crate::lenses::domain::LensTarget::Pr)
        })
    }

    /// A block's lenses, in order, answered or not.
    pub fn for_block(&self, slug: &Slug) -> Vec<Lensed<'_>> {
        self.in_group(LensGroup::Block, |answer| {
            matches!(&answer.target, crate::lenses::domain::LensTarget::Block { slug: s } if s == slug)
        })
    }

    /// A file's lenses, only where the session wrote one with something in it:
    /// which files a file lens applies to is the session's call, so a file
    /// without one is not missing anything.
    pub fn for_file(&self, path: &str) -> Vec<Lensed<'_>> {
        self.in_group(LensGroup::File, |answer| {
            matches!(&answer.target, crate::lenses::domain::LensTarget::File { path: p } if p == path)
        })
        .into_iter()
        .filter(|lensed| lensed.answer.is_some())
        .collect()
    }

    /// Whether some answer was written against this very content of the file:
    /// then the lenses already speak for it, and its diff can start folded.
    pub fn saw(&self, path: &str, blob: &str) -> bool {
        self.answers
            .iter()
            .any(|(answer, _)| answer.saw(path, blob))
    }

    /// The lens that asks a person to read this file themselves, if one does.
    pub fn read_yourself(&self, path: &str) -> Option<&str> {
        self.answers
            .iter()
            .find(|(answer, _)| answer.read.iter().any(|p| p == path))
            .and_then(|(answer, _)| self.lenses.iter().find(|l| l.id == answer.lens))
            .map(|lens| lens.title.as_str())
    }

    fn in_group(
        &self,
        group: LensGroup,
        aims_here: impl Fn(&LensAnswer) -> bool,
    ) -> Vec<Lensed<'_>> {
        self.lenses
            .iter()
            .filter(|lens| lens.group == group)
            .map(|lens| Lensed {
                lens,
                answer: self
                    .answers
                    .iter()
                    .find(|(answer, _)| answer.lens == lens.id && aims_here(answer))
                    .filter(|(answer, _)| !answer.is_empty())
                    .map(|(answer, freshness)| (answer, *freshness)),
            })
            .filter(|lensed| {
                // An empty answer is the session saying the lens does not
                // apply here; it is not drawn, and it is not missing either.
                lensed.answer.is_some()
                    || !self
                        .answers
                        .iter()
                        .any(|(a, _)| a.lens == lensed.lens.id && aims_here(a))
            })
            .collect()
    }
}

/// One lens on one target: what it asks, and what it said, if anything yet.
pub struct Lensed<'a> {
    pub lens: &'a LensDefinition,
    pub answer: Option<(&'a LensAnswer, Freshness)>,
}
