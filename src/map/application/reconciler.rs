//! Bringing an inherited map back into line with the code as it stands now.
//!
//! Separate from the version lifecycle because it changes for its own reasons:
//! how a note follows moving code, and what happens to a file that left the
//! review. Neither has anything to do with how versions are found or stored.

use crate::diff::application::{FileDiffs, ReviewScope};
use crate::diff::domain::FileDiff;
use crate::map::domain::{LineNote, LineRange, Orphan, OrphanReason, ReviewMap, ShiftOutcome};
use crate::shared::error::Result;

#[derive(Clone)]
pub struct MapReconciler {
    scope: ReviewScope,
    diffs: FileDiffs,
}

impl MapReconciler {
    pub fn new(scope: ReviewScope, diffs: FileDiffs) -> Self {
        Self { scope, diffs }
    }

    /// Move every line note onto the new commit, deactivating the ones whose
    /// code was rewritten.
    pub fn reanchor(&self, map: &mut ReviewMap, from: &str, to: &str) -> Result<()> {
        let mut orphans = Vec::new();

        for block in &mut map.blocks {
            for file in &mut block.files {
                if file.line_notes.is_empty() {
                    continue;
                }
                let Some(diff) = self.diffs.between(from, to, &file.path)? else {
                    continue; // file untouched: every note is still exactly right
                };

                let mut kept = Vec::new();
                for note in std::mem::take(&mut file.line_notes) {
                    match note.range.shift(&diff.hunks) {
                        ShiftOutcome::Unchanged => kept.push(note),
                        ShiftOutcome::Shifted(range) => kept.push(LineNote {
                            range,
                            text: note.text,
                        }),
                        ShiftOutcome::Overlapped => orphans.push(Orphan {
                            block: block.slug.clone(),
                            path: file.path.clone(),
                            old_range: note.range,
                            snapshot: Self::snapshot_of(&diff, note.range),
                            reason: OrphanReason::HunkOverlap,
                            text: note.text,
                        }),
                    }
                }
                file.line_notes = kept;
            }
        }

        map.orphans.extend(orphans);
        Ok(())
    }

    /// Files that left the review window cannot stay in the map. Their notes
    /// are kept as orphans so the prose can be moved rather than silently lost.
    pub fn prune_gone_files(&self, map: &mut ReviewMap) -> Result<()> {
        let scope = self.scope.get()?;
        let mut orphans = Vec::new();

        for block in &mut map.blocks {
            let slug = block.slug.clone();
            block.files.retain(|file| {
                if scope.contains(&file.path) {
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

        map.skim.retain(|s| scope.contains(&s.path));
        map.orphans.extend(orphans);
        Ok(())
    }

    /// The code the note used to cover. This — not the old line numbers — is
    /// what identifies the note afterwards: after a refactor, `82-116` may point
    /// at a completely different function, and restoring there would place a
    /// confident note on unrelated code.
    fn snapshot_of(diff: &FileDiff, range: LineRange) -> String {
        let lines: Vec<&str> = diff
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| matches!(l.old_number, Some(n) if n >= range.from && n <= range.to))
            .map(|l| l.content.as_str())
            .collect();

        let joined = lines.join("\n");
        if joined.chars().count() > 600 {
            joined.chars().take(600).collect::<String>() + "\n…"
        } else {
            joined
        }
    }
}
