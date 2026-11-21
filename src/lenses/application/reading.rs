//! The lenses as the screen reads them: which are on, what each said, and
//! whether that still matches the review.

use crate::diff::domain::FileChange;
use crate::lenses::domain::{Freshness, LensAnswer, LensDefinition, LensGroup, LensTarget};
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
    /// The lenses on one target, the way that target's lenses are listed.
    pub fn on(&self, target: &LensTarget) -> Vec<Lensed<'_>> {
        match target {
            LensTarget::Pr => self.for_pr(),
            LensTarget::Block { slug } => self.for_block(slug),
            LensTarget::File { path } => self.for_file(path),
        }
    }

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
    /// reader to mark what they never had to open.
    ///
    /// A block lens rewritten after the reader saw it still covers the files
    /// whose content did not change since: the rewrite was for another file of
    /// the block, and unreading the whole block for it was what made a small
    /// commit on a big branch take back almost everything read.
    pub fn covers(&self, map: &ReviewMap, progress: &Progress, change: &FileChange) -> bool {
        let path = change.path.as_str();
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
                        || progress.saw_file(&answer.seen_key(), path, &change.content_hash)
                })
            })
    }

    /// Whether the reader has read a file: ticked and still the same file, or
    /// covered by its block's lenses.
    ///
    /// The one place the rule is written. The row on screen, the block's count
    /// and what the pull request is told all ask this, so the three cannot come
    /// to disagree about what was read.
    pub fn is_read(&self, map: &ReviewMap, progress: &Progress, change: &FileChange) -> bool {
        // Comparing paths alone was what kept a file struck through after it
        // had changed under the mark, which is the one thing this is supposed
        // to catch.
        progress.is_current(&change.path, &change.content_hash)
            || self.covers(map, progress, change)
    }

    /// Every file under review the reader has read.
    pub fn read_paths(&self, snapshot: &ReviewSnapshot) -> Vec<String> {
        snapshot
            .scope
            .files
            .iter()
            .filter(|f| self.is_read(&snapshot.map, &snapshot.progress, f))
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

    /// A file under review at this content.
    fn at(path: &str, blob: &str) -> FileChange {
        FileChange {
            path: path.into(),
            old_path: None,
            status: crate::diff::domain::FileStatus::Modified,
            additions: 1,
            deletions: 0,
            content_hash: blob.into(),
        }
    }

    /// The block's lens, written against these files at these contents.
    fn block_lens(text: &str, written_at: &str, files: &[(&str, &str)]) -> (LensAnswer, Freshness) {
        let (mut lens, fresh) = answer("summary", LensTarget::Block { slug: slug("core") }, text);
        lens.written_at = written_at.into();
        lens.covers = files
            .iter()
            .map(|(path, blob)| crate::lenses::domain::Covered {
                path: path.to_string(),
                blob: blob.to_string(),
            })
            .collect();
        (lens, fresh)
    }

    fn see(progress: &mut Progress, lens: &LensAnswer) {
        progress.see(&lens.seen_key(), &lens.fingerprint(), lens.covered(), "now");
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
            !reading.covers(&map, &progress, &at("plain.rs", "blob")),
            "the block's lens is not seen yet"
        );
        see(&mut progress, &summary.0);

        assert!(reading.covers(&map, &progress, &at("plain.rs", "blob")));
        assert!(
            !reading.covers(&map, &progress, &at("tested.rs", "blob")),
            "a file with a lens of its own is read by its own tick"
        );
        assert!(
            !reading.covers(&map, &progress, &at("dense.rs", "blob")),
            "a file in focus is read by its own tick"
        );
    }

    #[test]
    fn a_file_is_read_while_its_tick_holds_or_its_block_covers_it() {
        // The one rule the row, the block's count and the pull request share.
        let mut map = map_with(&["core"]);
        map.add_file(&slug("core"), "a.rs", None, None).unwrap();
        let reading = LensReading {
            lenses: LensCatalog::default().enabled(),
            answers: vec![],
        };
        let change = |hash: &str| FileChange {
            path: "a.rs".into(),
            old_path: None,
            status: crate::diff::domain::FileStatus::Modified,
            additions: 1,
            deletions: 0,
            content_hash: hash.into(),
        };
        let mut progress = Progress::new();
        progress.mark("a.rs", "blob-1", "now");

        assert!(reading.is_read(&map, &progress, &change("blob-1")));
        assert!(
            !reading.is_read(&map, &progress, &change("blob-2")),
            "a tick on the file as it was is not a read of the file as it is"
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
            !reading.covers(&map, &Progress::new(), &at("plain.rs", "blob")),
            "with nothing written about the block, the file asks for its own tick"
        );
    }

    #[test]
    fn a_lens_the_session_keeps_after_a_commit_stays_seen_and_so_do_its_files() {
        // A keep is the session saying the words still hold. Asking for them
        // again unread every file the block's lenses had read for the reader.
        let mut map = map_with(&["core"]);
        map.add_file(&slug("core"), "a.rs", None, None).unwrap();
        let before = block_lens("how it is built", "first", &[("a.rs", "blob-1")]);
        let mut progress = Progress::new();
        see(&mut progress, &before.0);

        let kept = block_lens("how it is built", "second", &[("a.rs", "blob-1")]);
        let reading = LensReading {
            lenses: LensCatalog::default().enabled(),
            answers: vec![kept.clone()],
        };

        assert!(progress.has_seen(&kept.0.seen_key(), &kept.0.fingerprint()));
        assert!(reading.covers(&map, &progress, &at("a.rs", "blob-1")));
    }

    #[test]
    fn a_rewritten_block_lens_takes_back_only_the_files_that_changed() {
        // One file of a big block changes, the block's lens is rewritten for
        // it, and the other files of the block were still read by the reader.
        let mut map = map_with(&["core"]);
        for path in ["same.rs", "moved.rs"] {
            map.add_file(&slug("core"), path, None, None).unwrap();
        }
        let before = block_lens(
            "how it is built",
            "first",
            &[("same.rs", "s1"), ("moved.rs", "m1")],
        );
        let mut progress = Progress::new();
        see(&mut progress, &before.0);

        let rewritten = block_lens(
            "how it is built now",
            "second",
            &[("same.rs", "s1"), ("moved.rs", "m2")],
        );
        let reading = LensReading {
            lenses: LensCatalog::default().enabled(),
            answers: vec![rewritten.clone()],
        };

        assert!(
            !progress.has_seen(&rewritten.0.seen_key(), &rewritten.0.fingerprint()),
            "new words are read again"
        );
        assert!(reading.covers(&map, &progress, &at("same.rs", "s1")));
        assert!(
            !reading.covers(&map, &progress, &at("moved.rs", "m2")),
            "the file that changed asks to be read again"
        );

        progress.unsee(&rewritten.0.seen_key());
        assert!(
            !reading.covers(&map, &progress, &at("same.rs", "s1")),
            "a lens the reader took back reads nothing for them"
        );
    }
}
