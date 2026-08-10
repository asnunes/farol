//! Notes waiting for a decision after the code under them moved.
use super::*;

/// What should become of a line note once the code under it has moved.
///
/// The caller decides — it is the only one holding the diff — and the map
/// carries it out, so the orphan bookkeeping stays where the invariants are.
pub enum NoteFate {
    /// The lines did not move.
    Keep,
    /// The same code, further down or further up.
    MoveTo(LineRange),
    /// The code was rewritten; the prose outlives it.
    Orphan {
        /// The code the note used to cover, which is what identifies it now.
        snapshot: String,
        reason: OrphanReason,
    },
}

impl ReviewMap {
    pub fn orphans(&self) -> &[Orphan] {
        &self.orphans
    }

    /// Orphans belong to the version that produced them; the next one starts
    /// clean, so a graveyard nobody revisits cannot pile up.
    pub fn clear_orphans(&mut self) {
        self.orphans.clear();
    }

    /// Recompute every line note against code that moved.
    ///
    /// `decide` sees each note with the path it sits on and says what becomes
    /// of it. Everything else — dropping the note, recording the orphan against
    /// the right block, keeping the remaining notes in reading order — happens
    /// here, because it is the same bookkeeping every caller would otherwise
    /// have to repeat correctly.
    /// Generic over the caller's error because this method has none of its own:
    /// it decides nothing and refuses nothing, it only carries out what `decide`
    /// says. Forcing the caller to produce a `MapError` would make it wrap a git
    /// failure in the map's vocabulary, which it is not.
    pub fn reanchor_notes<E>(
        &mut self,
        mut decide: impl FnMut(&str, &LineNote) -> std::result::Result<NoteFate, E>,
    ) -> std::result::Result<(), E> {
        // Decide everything before changing anything. `decide` reads a diff to
        // make up its mind and that can fail; applying as we go would leave the
        // map with notes already lifted off their files and no record of them.
        let mut fates = Vec::new();
        for block in &self.blocks {
            for file in &block.files {
                for note in &file.line_notes {
                    fates.push(decide(&file.path, note)?);
                }
            }
        }

        // The same traversal, in the same order, over a shape nothing has
        // changed in between.
        let mut fates = fates.into_iter();
        let mut orphans = Vec::new();

        for block in &mut self.blocks {
            for file in &mut block.files {
                let mut kept = Vec::new();
                for note in std::mem::take(&mut file.line_notes) {
                    match fates.next().expect("one decision per note") {
                        NoteFate::Keep => kept.push(note),
                        NoteFate::MoveTo(range) => kept.push(LineNote {
                            range,
                            text: note.text,
                        }),
                        NoteFate::Orphan { snapshot, reason } => orphans.push(Orphan {
                            block: block.slug.clone(),
                            path: file.path.clone(),
                            old_range: note.range,
                            snapshot,
                            reason,
                            text: note.text,
                        }),
                    }
                }
                kept.sort_by_key(|n| n.range);
                file.line_notes = kept;
            }
        }

        self.orphans.extend(orphans);
        Ok(())
    }

    /// Drop everything the review no longer covers.
    ///
    /// A file that left the window cannot stay in the map, but its notes can:
    /// they are kept as orphans so the prose can be moved rather than silently
    /// lost. Skim entries have no prose, so they simply go.
    pub fn retain_covered(&mut self, covered: impl Fn(&str) -> bool) {
        let mut orphans = Vec::new();

        for block in &mut self.blocks {
            let slug = block.slug.clone();
            block.files.retain(|file| {
                if covered(&file.path) {
                    return true;
                }
                orphans.extend(file.line_notes.iter().map(|note| Orphan {
                    block: slug.clone(),
                    path: file.path.clone(),
                    old_range: note.range,
                    snapshot: String::new(),
                    reason: OrphanReason::FileRemoved,
                    text: note.text.clone(),
                }));
                false
            });
        }

        self.skim.retain(|s| covered(&s.path));
        self.orphans.extend(orphans);
    }

    pub fn take_orphan(&mut self, slug: &Slug, path: &str, range: LineRange) -> Result<Orphan> {
        let idx = self
            .orphans
            .iter()
            .position(|o| &o.block == slug && o.path == path && o.old_range == range)
            .ok_or_else(|| MapError::NoSuchOrphan {
                slug: slug.to_string(),
                path: path.to_string(),
                range,
            })?;
        Ok(self.orphans.remove(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{range, slug};

    /// One block, one file, two notes.
    fn mapped() -> ReviewMap {
        let mut m = ReviewMap::new("feature/x", "main", "abc123");
        m.add_block(&slug("core"), "t", "c", Position::End).unwrap();
        m.add_file(&slug("core"), "a.rs", None, None).unwrap();
        m.add_line_note(&slug("core"), "a.rs", range(10, 12), "first")
            .unwrap();
        m.add_line_note(&slug("core"), "a.rs", range(40, 42), "second")
            .unwrap();
        m
    }

    fn notes(m: &ReviewMap) -> Vec<(LineRange, String)> {
        m.block(&slug("core")).unwrap().files[0]
            .line_notes
            .iter()
            .map(|n| (n.range, n.text.clone()))
            .collect()
    }

    #[test]
    fn a_note_the_caller_keeps_is_left_exactly_as_it_was() {
        let mut m = mapped();

        m.reanchor_notes(|_, _| Ok::<_, MapError>(NoteFate::Keep))
            .unwrap();

        assert_eq!(notes(&m).len(), 2);
        assert!(m.orphans().is_empty());
    }

    #[test]
    fn a_moved_note_carries_its_prose_to_the_new_span() {
        let mut m = mapped();

        m.reanchor_notes(|_, note| {
            Ok::<_, MapError>(if note.range == range(10, 12) {
                NoteFate::MoveTo(range(60, 62))
            } else {
                NoteFate::Keep
            })
        })
        .unwrap();

        assert!(notes(&m).contains(&(range(60, 62), "first".to_string())));
    }

    #[test]
    fn moved_notes_come_back_in_reading_order() {
        // A note that slid past another would otherwise be listed above it.
        let mut m = mapped();

        m.reanchor_notes(|_, note| {
            Ok::<_, MapError>(if note.range == range(10, 12) {
                NoteFate::MoveTo(range(90, 92))
            } else {
                NoteFate::Keep
            })
        })
        .unwrap();

        let spans: Vec<_> = notes(&m).into_iter().map(|(r, _)| r).collect();
        assert_eq!(spans, vec![range(40, 42), range(90, 92)]);
    }

    #[test]
    fn an_orphaned_note_leaves_the_file_and_lands_with_everything_needed_to_place_it() {
        let mut m = mapped();

        m.reanchor_notes(|_, note| {
            Ok::<_, MapError>(if note.range == range(10, 12) {
                NoteFate::Orphan {
                    snapshot: "let timeout = 180;".into(),
                    reason: OrphanReason::HunkOverlap,
                }
            } else {
                NoteFate::Keep
            })
        })
        .unwrap();

        assert_eq!(notes(&m).len(), 1, "the orphaned one is off the file");
        let o = &m.orphans()[0];
        assert_eq!(o.block, slug("core"));
        assert_eq!(o.path, "a.rs");
        assert_eq!(o.old_range, range(10, 12));
        assert_eq!(
            o.text, "first",
            "the prose is the whole point of keeping it"
        );
        assert_eq!(o.snapshot, "let timeout = 180;");
        assert_eq!(o.reason, OrphanReason::HunkOverlap);
    }

    #[test]
    fn the_caller_is_told_which_file_each_note_sits_on() {
        // It needs the path to fetch the right diff.
        let mut m = mapped();
        m.add_file(&slug("core"), "b.rs", None, None).unwrap();
        m.add_line_note(&slug("core"), "b.rs", range(5, 6), "on b")
            .unwrap();

        let mut seen = Vec::new();
        m.reanchor_notes(|path, _| {
            seen.push(path.to_string());
            Ok::<_, MapError>(NoteFate::Keep)
        })
        .unwrap();

        seen.sort();
        seen.dedup();
        assert_eq!(seen, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn a_failure_partway_through_leaves_every_note_where_it_was() {
        // The caller reads a diff to decide, and that can fail. Losing the
        // prose because git hiccuped would be the expensive kind of loss.
        let mut m = mapped();

        let err = m.reanchor_notes(|_, note| {
            if note.range == range(40, 42) {
                return Err("cannot read the diff");
            }
            Ok(NoteFate::MoveTo(range(60, 62)))
        });

        assert!(err.is_err());
        assert_eq!(
            notes(&m),
            vec![
                (range(10, 12), "first".to_string()),
                (range(40, 42), "second".to_string())
            ],
            "a half-applied reanchor is worse than none"
        );
    }

    // ---- retain_covered -------------------------------------------------

    #[test]
    fn a_file_still_under_review_keeps_its_notes() {
        let mut m = mapped();

        m.retain_covered(|_| true);

        assert_eq!(notes(&m).len(), 2);
        assert!(m.orphans().is_empty());
    }

    #[test]
    fn a_file_that_left_the_review_goes_but_its_prose_stays() {
        let mut m = mapped();

        m.retain_covered(|_| false);

        assert!(m.block(&slug("core")).unwrap().files.is_empty());
        assert_eq!(m.orphans().len(), 2);
        assert!(
            m.orphans()
                .iter()
                .all(|o| o.reason == OrphanReason::FileRemoved),
            "the file is gone, not the code under the note"
        );
        assert_eq!(m.orphans()[0].text, "first");
    }

    #[test]
    fn only_the_files_that_left_are_dropped() {
        let mut m = mapped();
        m.add_file(&slug("core"), "b.rs", None, None).unwrap();

        m.retain_covered(|path| path == "a.rs");

        let paths: Vec<&str> = m
            .block(&slug("core"))
            .unwrap()
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert_eq!(paths, vec!["a.rs"]);
    }

    #[test]
    fn a_skim_entry_that_left_the_review_simply_goes() {
        // It carries no prose, so there is nothing to keep.
        let mut m = mapped();
        m.add_skim("Cargo.lock", "generated", None).unwrap();

        m.retain_covered(|path| path != "Cargo.lock");

        assert!(m.skim().is_empty());
        assert!(m.orphans().is_empty());
    }

    #[test]
    fn pruning_adds_to_the_orphans_already_there_rather_than_replacing_them() {
        let mut m = mapped();
        m.reanchor_notes(|_, note| {
            Ok::<_, MapError>(if note.range == range(10, 12) {
                NoteFate::Orphan {
                    snapshot: String::new(),
                    reason: OrphanReason::HunkOverlap,
                }
            } else {
                NoteFate::Keep
            })
        })
        .unwrap();

        m.retain_covered(|_| false);

        assert_eq!(m.orphans().len(), 2, "one from each step");
    }

    // ---- take_orphan ----------------------------------------------------

    #[test]
    fn taking_an_orphan_hands_it_over_and_removes_it_from_the_list() {
        let mut m = mapped();
        m.reanchor_notes(|_, _| {
            Ok::<_, MapError>(NoteFate::Orphan {
                snapshot: String::new(),
                reason: OrphanReason::HunkOverlap,
            })
        })
        .unwrap();

        let taken = m.take_orphan(&slug("core"), "a.rs", range(10, 12)).unwrap();

        assert_eq!(taken.text, "first");
        assert_eq!(m.orphans().len(), 1, "the other one is untouched");
    }

    #[test]
    fn taking_an_orphan_that_is_not_there_is_refused() {
        let mut m = mapped();

        assert!(m.take_orphan(&slug("core"), "a.rs", range(10, 12)).is_err());
    }

    #[test]
    fn orphans_are_cleared_wholesale_when_a_version_is_left_behind() {
        // They belong to the version that produced them; carrying them forever
        // would pile up a graveyard nobody revisits.
        let mut m = mapped();
        m.reanchor_notes(|_, _| {
            Ok::<_, MapError>(NoteFate::Orphan {
                snapshot: String::new(),
                reason: OrphanReason::HunkOverlap,
            })
        })
        .unwrap();

        m.clear_orphans();

        assert!(m.orphans().is_empty());
    }
}
