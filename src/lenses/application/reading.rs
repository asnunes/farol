//! The lenses as the screen reads them: which are on, what each said, and
//! whether that still matches the review.

use crate::lenses::domain::{Freshness, LensAnswer, LensDefinition, LensGroup};
use crate::map::application::ReviewSnapshot;
use crate::map::domain::{ReviewMap, Slug};
use crate::progress::domain::Progress;

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

    /// Whether a file counts as read without a tick of its own: it is not in
    /// focus, no lens of its own says anything, and every lens of its block
    /// is written and seen. Such a file has nothing to read that the block's
    /// lenses did not already say, so asking for its tick would be asking the
    /// reader to mark what they never had to open. Nothing is stored: a block
    /// lens rewritten is a block lens unseen, and the files follow it.
    pub fn covers(&self, map: &ReviewMap, progress: &Progress, path: &str) -> bool {
        let Some(block) = map.block_of(path) else {
            return false;
        };
        if block.file(path).is_some_and(|f| f.focus.is_some()) || !self.for_file(path).is_empty() {
            return false;
        }
        let lenses = self.for_block(&block.slug);
        !lenses.is_empty()
            && lenses.iter().all(|l| {
                l.answer.is_some_and(|(answer, _)| {
                    progress.has_seen(&answer.seen_key(), &answer.fingerprint())
                })
            })
    }

    /// Every file under review the reader has read: ticked and unchanged since,
    /// or covered by its block's lenses.
    pub fn read_paths(&self, snapshot: &ReviewSnapshot) -> Vec<String> {
        snapshot
            .scope
            .files
            .iter()
            .filter(|f| {
                snapshot.progress.is_current(&f.path, &f.content_hash)
                    || self.covers(&snapshot.map, &snapshot.progress, &f.path)
            })
            .map(|f| f.path.clone())
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lenses::domain::{LensCatalog, LensId, LensTarget};
    use crate::testing::{map_with, slug};

    fn answer(lens: &str, target: LensTarget, text: &str) -> (LensAnswer, Freshness) {
        let answer = LensAnswer {
            lens: LensId::parse(lens).unwrap(),
            target,
            text: text.into(),
            written_at: "head".into(),
            covers: vec![],
            prompt_hash: "p".into(),
            citations: vec![],
            open: None,
        };
        (answer, Freshness::Written)
    }

    #[test]
    fn a_file_with_nothing_of_its_own_is_read_once_its_block_lenses_are_seen() {
        let mut map = map_with(&["core"]);
        for path in ["plain.rs", "tested.rs", "dense.rs"] {
            map.add_file(&slug("core"), path, None, None).unwrap();
        }
        map.focus(&slug("core"), "dense.rs", Some("dense".into()))
            .unwrap();
        let summary = answer(
            "summary",
            LensTarget::Block { slug: slug("core") },
            "how it is built",
        );
        let reading = LensReading {
            lenses: LensCatalog::default().enabled(),
            answers: vec![
                summary.clone(),
                answer(
                    "tests",
                    LensTarget::File {
                        path: "tested.rs".into(),
                    },
                    "- retries",
                ),
            ],
        };
        let mut progress = Progress::new();

        assert!(
            !reading.covers(&map, &progress, "plain.rs"),
            "the block's lens is not seen yet"
        );
        progress.see(&summary.0.seen_key(), &summary.0.fingerprint(), "now");

        assert!(reading.covers(&map, &progress, "plain.rs"));
        assert!(
            !reading.covers(&map, &progress, "tested.rs"),
            "a file with a lens of its own is read by its own tick"
        );
        assert!(
            !reading.covers(&map, &progress, "dense.rs"),
            "a file in focus is read by its own tick"
        );
    }

    #[test]
    fn a_block_with_no_lens_written_covers_nothing() {
        let mut map = map_with(&["core"]);
        map.add_file(&slug("core"), "plain.rs", None, None).unwrap();
        let reading = LensReading {
            lenses: LensCatalog::default().enabled(),
            answers: vec![],
        };

        assert!(
            !reading.covers(&map, &Progress::new(), "plain.rs"),
            "with nothing written about the block, the file asks for its own tick"
        );
    }
}
