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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::LineRange;
    use crate::testing::{FakeDiffSource, hunk, hunk_with_lines, map_with_note, reconciler, slug};

    fn range(from: u32, to: u32) -> LineRange {
        LineRange::new(from, to).unwrap()
    }

    fn notes_of(map: &ReviewMap) -> Vec<LineRange> {
        map.block(&slug("core"))
            .unwrap()
            .file("a.rs")
            .unwrap()
            .line_notes
            .iter()
            .map(|n| n.range)
            .collect()
    }

    #[test]
    fn a_file_that_did_not_change_leaves_its_notes_exactly_where_they_are() {
        // No diff between the two commits, so nothing to recompute.
        let source = FakeDiffSource::with_paths(&["a.rs"]);
        let mut map = map_with_note("old", range(40, 45), "still true");

        reconciler(source).reanchor(&mut map, "old", "new").unwrap();

        assert_eq!(notes_of(&map), vec![range(40, 45)]);
        assert!(map.orphans.is_empty());
    }

    #[test]
    fn a_change_above_the_note_slides_it_down() {
        let source = FakeDiffSource::with_paths(&["a.rs"]).changed_between(
            "old",
            "new",
            "a.rs",
            vec![hunk(1, 0, 5)],
        );
        let mut map = map_with_note("old", range(40, 45), "still true");

        reconciler(source).reanchor(&mut map, "old", "new").unwrap();

        assert_eq!(notes_of(&map), vec![range(45, 50)]);
        assert!(map.orphans.is_empty());
    }

    #[test]
    fn a_change_inside_the_note_deactivates_it_and_keeps_the_code_it_covered() {
        let source = FakeDiffSource::with_paths(&["a.rs"]).changed_between(
            "old",
            "new",
            "a.rs",
            vec![hunk_with_lines(40, &["const timeout = 180;", "let x = 1;"])],
        );
        let mut map = map_with_note("old", range(40, 41), "expensive prose");

        reconciler(source).reanchor(&mut map, "old", "new").unwrap();

        assert!(notes_of(&map).is_empty());
        assert_eq!(map.orphans.len(), 1);
        assert_eq!(map.orphans[0].text, "expensive prose");
        assert_eq!(map.orphans[0].reason, OrphanReason::HunkOverlap);
        assert_eq!(
            map.orphans[0].snapshot, "const timeout = 180;\nlet x = 1;",
            "the snapshot is what identifies the note afterwards, not the old range"
        );
        assert_eq!(map.orphans[0].old_range, range(40, 41));
    }

    #[test]
    fn several_notes_on_one_file_are_decided_independently() {
        // One sits above the change and slides; one sits inside it and does not.
        let source = FakeDiffSource::with_paths(&["a.rs"]).changed_between(
            "old",
            "new",
            "a.rs",
            vec![hunk_with_lines(50, &["rewritten"])],
        );
        let mut map = map_with_note("old", range(80, 82), "below the change");
        map.add_line_note(&slug("core"), "a.rs", range(50, 50), "inside the change")
            .unwrap();

        reconciler(source).reanchor(&mut map, "old", "new").unwrap();

        assert_eq!(notes_of(&map), vec![range(80, 82)]);
        assert_eq!(map.orphans.len(), 1);
        assert_eq!(map.orphans[0].text, "inside the change");
    }

    #[test]
    fn a_snapshot_longer_than_the_cap_is_truncated_rather_than_dumped() {
        let long: Vec<String> = (1..=200).map(|i| format!("line number {i}")).collect();
        let refs: Vec<&str> = long.iter().map(String::as_str).collect();
        let source = FakeDiffSource::with_paths(&["a.rs"]).changed_between(
            "old",
            "new",
            "a.rs",
            vec![hunk_with_lines(1, &refs)],
        );
        let mut map = map_with_note("old", range(1, 200), "covers a lot");

        reconciler(source).reanchor(&mut map, "old", "new").unwrap();

        let snapshot = &map.orphans[0].snapshot;
        assert!(
            snapshot.chars().count() <= 602,
            "got {} chars",
            snapshot.chars().count()
        );
        assert!(
            snapshot.ends_with('…'),
            "truncation should be visible: {snapshot}"
        );
    }

    #[test]
    fn a_file_no_longer_under_review_is_dropped_and_its_notes_kept() {
        // The branch reverted the change to a.rs, so it left the window.
        let source = FakeDiffSource::with_paths(&["b.rs"]);
        let mut map = map_with_note("old", range(3, 4), "worth moving");

        reconciler(source).prune_gone_files(&mut map).unwrap();

        assert!(map.block(&slug("core")).unwrap().files.is_empty());
        assert_eq!(map.orphans.len(), 1);
        assert_eq!(map.orphans[0].reason, OrphanReason::FileRemoved);
        assert_eq!(map.orphans[0].text, "worth moving");
    }

    #[test]
    fn pruning_also_drops_skim_entries_for_files_that_left() {
        let source = FakeDiffSource::with_paths(&["a.rs"]);
        let mut map = map_with_note("old", range(1, 2), "n");
        map.add_skim("gone.lock", "generated", None).unwrap();
        map.add_skim("a.rs", "still here", None).unwrap();

        reconciler(source).prune_gone_files(&mut map).unwrap();

        let paths: Vec<&str> = map.skim.iter().map(|s| s.path.as_str()).collect();
        assert_eq!(paths, vec!["a.rs"]);
    }

    #[test]
    fn a_file_still_under_review_survives_pruning_untouched() {
        let source = FakeDiffSource::with_paths(&["a.rs"]);
        let mut map = map_with_note("old", range(3, 4), "keep me");

        reconciler(source).prune_gone_files(&mut map).unwrap();

        assert_eq!(notes_of(&map), vec![range(3, 4)]);
        assert!(map.orphans.is_empty());
    }
}
