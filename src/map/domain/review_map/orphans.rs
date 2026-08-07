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
    pub fn reanchor_notes(
        &mut self,
        mut decide: impl FnMut(&str, &LineNote) -> Result<NoteFate>,
    ) -> Result<()> {
        let mut orphans = Vec::new();

        for block in &mut self.blocks {
            for file in &mut block.files {
                let mut kept = Vec::new();
                for note in std::mem::take(&mut file.line_notes) {
                    match decide(&file.path, &note)? {
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
            .ok_or_else(|| Error::NoSuchOrphan {
                slug: slug.to_string(),
                path: path.to_string(),
                range,
            })?;
        Ok(self.orphans.remove(idx))
    }
}
