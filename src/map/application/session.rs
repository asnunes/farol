//! Everything you can do to a map, hung off the two things it always needs:
//! where the code is and where the map is stored.
//!
//! Deriving rather than regenerating exists for one reason: whoever is reading
//! may be halfway through. Rebuilding from scratch reshuffles block boundaries
//! and names, and they lose their place even though the new map is just as
//! good. A typo commit must not cost that.

use crate::diff::domain::{DiffSource, FileDiff};
use crate::map::domain::{
    LineRange, MapRepository, Orphan, OrphanReason, Position, ReviewMap, ShiftOutcome, WORKING,
};
use crate::shared::error::{Error, Result};

pub struct MapSession<'a> {
    source: &'a dyn DiffSource,
    repo: &'a dyn MapRepository,
}

pub struct Derived {
    pub map: ReviewMap,
    /// True when this call created the version rather than finding it.
    pub created: bool,
}

pub enum ResetOutcome {
    /// Deleted; the named version is current again, or nothing is.
    Deleted {
        fell_back_to: Option<String>,
    },
    NothingToDelete,
}

#[derive(Debug, Default)]
pub struct CheckReport {
    pub uncovered: Vec<String>,
    pub pending_orphans: usize,
    pub commits_behind: u32,
}

impl CheckReport {
    pub fn passed(&self) -> bool {
        self.uncovered.is_empty() && self.pending_orphans == 0
    }
}

impl<'a> MapSession<'a> {
    pub fn new(source: &'a dyn DiffSource, repo: &'a dyn MapRepository) -> Self {
        Self { source, repo }
    }

    pub fn source(&self) -> &'a dyn DiffSource {
        self.source
    }

    /// The version key for where we are: a commit, or the working-tree marker.
    pub fn target(&self) -> Result<String> {
        let scope = self.source.scope()?;
        Ok(if scope.dirty {
            WORKING.to_string()
        } else {
            scope.head_sha.clone()
        })
    }

    /// Idempotent: creates the version for the current commit if it is missing,
    /// returns the existing one otherwise. Either way the caller can read the
    /// pending orphans off the map, so a run that died halfway simply picks up.
    pub fn derive(&self) -> Result<Derived> {
        let target = self.target()?;

        if let Some(existing) = self.repo.load_at(&target)? {
            return Ok(Derived {
                map: existing,
                created: false,
            });
        }

        let scope = self.source.scope()?;
        let mut map = match self.find_parent(&target)? {
            Some((parent_sha, parent_map)) => {
                let mut m = parent_map;
                m.parent = Some(parent_sha.clone());
                m.generated_at = target.clone();
                m.branch = scope.branch.clone();
                m.base = scope.base_ref.clone();
                // Orphans belong to the version that produced them; the next one
                // starts clean. Carrying them forever would pile up a graveyard
                // nobody revisits.
                m.orphans.clear();
                self.reanchor(&mut m, &parent_sha, &target)?;
                m
            }
            None => ReviewMap::new(&scope.branch, &scope.base_ref, &target),
        };

        self.prune_gone_files(&mut map)?;
        self.repo.save(&map)?;

        // A working map that has been absorbed must not keep being picked as
        // the parent of every future commit.
        if !scope.dirty && map.parent.as_deref() == Some(WORKING) {
            self.repo.delete(WORKING)?;
        }

        Ok(Derived { map, created: true })
    }

    /// The map that belongs to where we are now, falling back to the newest
    /// ancestor that has one — which is what makes "two commits behind" a state
    /// rather than an absence.
    pub fn current(&self) -> Result<Option<ReviewMap>> {
        let target = self.target()?;
        if let Some(map) = self.repo.load_at(&target)? {
            return Ok(Some(map));
        }
        Ok(self.nearest_ancestor(&target)?.map(|(_, map)| map))
    }

    /// Same as `current`, but says so instead of returning nothing.
    pub fn require_current(&self) -> Result<ReviewMap> {
        self.current()?.ok_or_else(|| Error::NoMap {
            branch: self
                .source
                .scope()
                .map(|s| s.branch.clone())
                .unwrap_or_default(),
        })
    }

    pub fn behind(&self, map: &ReviewMap) -> u32 {
        self.source.commits_ahead_of(&map.generated_at).unwrap_or(0)
    }

    /// Load the current version, apply `f`, store it back.
    pub fn edit<F>(&self, f: F) -> Result<ReviewMap>
    where
        F: FnOnce(&mut ReviewMap) -> Result<()>,
    {
        let mut map = self.derive()?.map;
        f(&mut map)?;
        self.repo.save(&map)?;
        Ok(map)
    }

    pub fn reset(&self) -> Result<ResetOutcome> {
        let target = self.target()?;
        if self.repo.load_at(&target)?.is_none() {
            return Ok(ResetOutcome::NothingToDelete);
        }
        self.repo.delete(&target)?;
        Ok(ResetOutcome::Deleted {
            fell_back_to: self.current()?.map(|m| m.generated_at),
        })
    }

    /// The map's self-test. It exists so "every file shows up somewhere" does
    /// not depend on the model remembering the rule: a file nobody assigned is
    /// not merely undocumented, it is invisible, because the sidebar is built
    /// from the map.
    pub fn check(&self, map: &ReviewMap) -> Result<CheckReport> {
        let scope = self.source.scope()?;
        let covered = map.covered_paths();
        Ok(CheckReport {
            uncovered: scope
                .files
                .iter()
                .map(|f| f.path.clone())
                .filter(|p| !covered.contains(p))
                .collect(),
            pending_orphans: map.orphans.len(),
            commits_behind: self.behind(map),
        })
    }

    /// Reject a path the reviewer will never be shown, and say what was
    /// probably meant. Hallucinated paths are the most common way an agent gets
    /// this wrong, and a suggestion turns a rejection into a self-correction.
    pub fn require_in_scope(&self, path: &str) -> Result<()> {
        let scope = self.source.scope()?;
        if scope.contains(path) {
            return Ok(());
        }
        Err(scope.reject(path))
    }

    pub fn require_range_in_file(&self, path: &str, range: LineRange) -> Result<()> {
        range.require_within(path, self.source.file_line_count(path)?)
    }

    // ---- use cases ----------------------------------------------------
    //
    // Each one validates what it needs before touching the map, so a caller
    // cannot skip a check by forgetting to make it. The entry points parse
    // arguments and print; they do not decide what is allowed.

    pub fn add_block(
        &self,
        slug: &str,
        title: &str,
        context: &str,
        position: Position,
        paths: &[String],
    ) -> Result<ReviewMap> {
        for path in paths {
            self.require_in_scope(path)?;
        }
        self.edit(|map| {
            map.add_block(slug, title, context, position)?;
            for path in paths {
                map.add_file(slug, path, None, None)?;
            }
            Ok(())
        })
    }

    pub fn update_block(
        &self,
        slug: &str,
        title: Option<String>,
        context: Option<String>,
    ) -> Result<ReviewMap> {
        self.edit(|map| map.update_block(slug, title, context))
    }

    pub fn remove_block(&self, slug: &str) -> Result<ReviewMap> {
        self.edit(|map| map.remove_block(slug))
    }

    pub fn move_block(&self, slug: &str, position: Position) -> Result<ReviewMap> {
        self.edit(|map| map.move_block(slug, position))
    }

    pub fn add_file(
        &self,
        slug: &str,
        path: &str,
        note: Option<String>,
        after: Option<&str>,
    ) -> Result<ReviewMap> {
        self.require_in_scope(path)?;
        self.edit(|map| map.add_file(slug, path, note, after))
    }

    pub fn update_file(&self, slug: &str, path: &str, note: String) -> Result<ReviewMap> {
        self.edit(|map| map.update_file(slug, path, Some(note)))
    }

    pub fn remove_file(&self, slug: &str, path: &str) -> Result<ReviewMap> {
        self.edit(|map| map.remove_file(slug, path))
    }

    pub fn add_line_note(
        &self,
        slug: &str,
        path: &str,
        range: LineRange,
        note: String,
    ) -> Result<ReviewMap> {
        self.require_in_scope(path)?;
        self.require_range_in_file(path, range)?;
        self.edit(|map| map.add_line_note(slug, path, range, note))
    }

    pub fn update_line_note(
        &self,
        slug: &str,
        path: &str,
        range: LineRange,
        note: String,
    ) -> Result<ReviewMap> {
        self.edit(|map| map.update_line_note(slug, path, range, note))
    }

    pub fn remove_line_note(&self, slug: &str, path: &str, range: LineRange) -> Result<ReviewMap> {
        self.edit(|map| map.remove_line_note(slug, path, range))
    }

    /// Bring a deactivated note back at the place its code moved to. The new
    /// range is checked against the file, because a restore pointing past the
    /// end would render nowhere.
    pub fn restore_note(
        &self,
        slug: &str,
        path: &str,
        old: LineRange,
        new: LineRange,
    ) -> Result<ReviewMap> {
        self.require_in_scope(path)?;
        self.require_range_in_file(path, new)?;
        self.edit(|map| {
            let orphan = map.take_orphan(slug, path, old)?;
            map.add_line_note(slug, path, new, orphan.text)
        })
    }

    pub fn discard_note(&self, slug: &str, path: &str, old: LineRange) -> Result<ReviewMap> {
        self.edit(|map| map.take_orphan(slug, path, old).map(|_| ()))
    }

    pub fn add_skim(&self, path: &str, reason: &str, block: Option<String>) -> Result<ReviewMap> {
        self.require_in_scope(path)?;
        self.edit(|map| map.add_skim(path, reason, block))
    }

    pub fn remove_skim(&self, path: &str) -> Result<ReviewMap> {
        self.edit(|map| map.remove_skim(path))
    }

    // ---- derivation internals -----------------------------------------

    /// The map to inherit from: uncommitted work first, then the newest stored
    /// commit that is an ancestor of where we are now.
    fn find_parent(&self, target: &str) -> Result<Option<(String, ReviewMap)>> {
        if target != WORKING
            && let Some(wip) = self.repo.load_at(WORKING)?
        {
            return Ok(Some((WORKING.to_string(), wip)));
        }
        self.nearest_ancestor(target)
    }

    fn nearest_ancestor(&self, target: &str) -> Result<Option<(String, ReviewMap)>> {
        let mut best: Option<(u32, String)> = None;
        for sha in self.repo.stored_shas()? {
            if sha == target || sha == WORKING || !self.source.is_ancestor(&sha)? {
                continue;
            }
            let distance = self.source.commits_ahead_of(&sha)?;
            if best.as_ref().map(|(d, _)| distance < *d).unwrap_or(true) {
                best = Some((distance, sha));
            }
        }
        match best {
            Some((_, sha)) => Ok(self.repo.load_at(&sha)?.map(|m| (sha, m))),
            None => Ok(None),
        }
    }

    /// Move every line note onto the new commit, deactivating the ones whose
    /// code was rewritten.
    fn reanchor(&self, map: &mut ReviewMap, from: &str, to: &str) -> Result<()> {
        let mut orphans = Vec::new();

        for block in &mut map.blocks {
            for file in &mut block.files {
                if file.line_notes.is_empty() {
                    continue;
                }
                let Some(diff) = self.source.file_diff_between(from, to, &file.path)? else {
                    continue; // file untouched: every note is still exactly right
                };

                let mut kept = Vec::new();
                for note in std::mem::take(&mut file.line_notes) {
                    match note.range.shift(&diff.hunks) {
                        ShiftOutcome::Unchanged => kept.push(note),
                        ShiftOutcome::Shifted(range) => kept.push(crate::map::domain::LineNote {
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

    /// Files that left the review window cannot stay in the map. Their notes
    /// are kept as orphans so the prose can be moved rather than silently lost.
    fn prune_gone_files(&self, map: &mut ReviewMap) -> Result<()> {
        let scope = self.source.scope()?;
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
}

/// Where a newly added block or file goes, from the two CLI flags.
pub fn position_from(before: Option<String>, after: Option<String>) -> Position {
    match (before, after) {
        (Some(b), _) => Position::Before(b),
        (None, Some(a)) => Position::After(a),
        (None, None) => Position::End,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::{LineRange, Position};
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, hunk, hunk_with_lines};

    fn mapped(sha: &str, range: LineRange, text: &str) -> ReviewMap {
        let mut map = ReviewMap::new("feature/x", "main", sha);
        map.add_block("core", "Core", "why", Position::End).unwrap();
        map.add_file("core", "a.rs", None, None).unwrap();
        map.add_line_note("core", "a.rs", range, text).unwrap();
        map
    }

    #[test]
    fn deriving_twice_on_the_same_commit_returns_the_existing_version() {
        let source = FakeDiffSource::with_paths(&["a.rs"]).on_commit("head");
        let repo = InMemoryMapRepository::new();
        let session = MapSession::new(&source, &repo);

        assert!(session.derive().unwrap().created);
        assert!(
            !session.derive().unwrap().created,
            "a second call must find the version, not make another"
        );
    }

    #[test]
    fn a_note_far_from_the_change_slides_instead_of_being_deactivated() {
        // Five lines appear at the top; nothing the note described was touched.
        let source = FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("new")
            .with_ancestors(&["old"])
            .at_distance("old", 1)
            .changed_between("old", "new", "a.rs", vec![hunk(1, 0, 5)]);

        let repo = InMemoryMapRepository::new();
        repo.seed(mapped("old", LineRange::new(40, 45).unwrap(), "still true"));

        let map = MapSession::new(&source, &repo).derive().unwrap().map;
        let notes = &map.block("core").unwrap().file("a.rs").unwrap().line_notes;
        assert_eq!(notes[0].range, LineRange::new(45, 50).unwrap());
        assert!(map.orphans.is_empty());
    }

    #[test]
    fn a_note_whose_code_was_rewritten_is_deactivated_with_a_snapshot() {
        let source = FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("new")
            .with_ancestors(&["old"])
            .at_distance("old", 1)
            .changed_between(
                "old",
                "new",
                "a.rs",
                vec![hunk_with_lines(40, &["const timeout = 180;"])],
            );

        let repo = InMemoryMapRepository::new();
        repo.seed(mapped(
            "old",
            LineRange::new(40, 40).unwrap(),
            "expensive prose",
        ));

        let map = MapSession::new(&source, &repo).derive().unwrap().map;
        assert!(
            map.block("core")
                .unwrap()
                .file("a.rs")
                .unwrap()
                .line_notes
                .is_empty()
        );
        assert_eq!(map.orphans.len(), 1);
        assert_eq!(map.orphans[0].text, "expensive prose");
        assert_eq!(
            map.orphans[0].snapshot, "const timeout = 180;",
            "the snapshot is what identifies the note afterwards, not the old range"
        );
    }

    #[test]
    fn orphans_do_not_survive_into_the_next_version() {
        let source = FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("newer")
            .with_ancestors(&["new"])
            .at_distance("new", 1);

        let repo = InMemoryMapRepository::new();
        let mut stale = mapped("new", LineRange::new(1, 2).unwrap(), "x");
        stale.orphans.push(crate::map::domain::Orphan {
            block: "core".into(),
            path: "a.rs".into(),
            old_range: LineRange::new(9, 9).unwrap(),
            snapshot: String::new(),
            reason: OrphanReason::HunkOverlap,
            text: "nobody decided".into(),
        });
        repo.seed(stale);

        let map = MapSession::new(&source, &repo).derive().unwrap().map;
        assert!(
            map.orphans.is_empty(),
            "carrying them forever would pile up a graveyard nobody revisits"
        );
    }

    #[test]
    fn a_file_that_left_the_review_is_dropped_and_its_notes_kept() {
        let source = FakeDiffSource::with_paths(&["b.rs"])
            .on_commit("new")
            .with_ancestors(&["old"])
            .at_distance("old", 1);

        let repo = InMemoryMapRepository::new();
        repo.seed(mapped("old", LineRange::new(3, 4).unwrap(), "worth moving"));

        let map = MapSession::new(&source, &repo).derive().unwrap().map;
        assert!(map.block("core").unwrap().files.is_empty());
        assert_eq!(map.orphans[0].reason, OrphanReason::FileRemoved);
        assert_eq!(map.orphans[0].text, "worth moving");
    }

    #[test]
    fn a_use_case_refuses_a_path_outside_the_review_on_its_own() {
        // The point of moving this out of the CLI: a second caller cannot skip
        // the check by forgetting to make it.
        let source = FakeDiffSource::with_paths(&["a.rs"]).on_commit("head");
        let repo = InMemoryMapRepository::new();
        let session = MapSession::new(&source, &repo);
        session
            .add_block("core", "t", "c", Position::End, &[])
            .unwrap();

        let err = session
            .add_file("core", "nowhere.rs", None, None)
            .unwrap_err();
        assert!(matches!(err, Error::PathOutOfScope { .. }), "{err:?}");

        let err = session
            .add_block(
                "other",
                "t",
                "c",
                Position::End,
                &["nowhere.rs".to_string()],
            )
            .unwrap_err();
        assert!(matches!(err, Error::PathOutOfScope { .. }), "{err:?}");
        assert!(
            session.require_current().unwrap().block("other").is_none(),
            "a rejected use case must not leave half of itself behind"
        );
    }

    #[test]
    fn adding_a_block_with_files_is_one_step_for_the_caller() {
        let source = FakeDiffSource::with_paths(&["a.rs", "b.rs"]).on_commit("head");
        let repo = InMemoryMapRepository::new();
        let session = MapSession::new(&source, &repo);

        let map = session
            .add_block(
                "core",
                "The change",
                "why",
                Position::End,
                &["a.rs".to_string(), "b.rs".to_string()],
            )
            .unwrap();

        let files: Vec<_> = map
            .block("core")
            .unwrap()
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert_eq!(files, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn check_fails_on_a_file_nobody_assigned() {
        let source = FakeDiffSource::with_paths(&["a.rs", "forgotten.rs"]).on_commit("head");
        let repo = InMemoryMapRepository::new();
        let session = MapSession::new(&source, &repo);
        session
            .edit(|map| {
                map.add_block("core", "t", "c", Position::End)?;
                map.add_file("core", "a.rs", None, None)
            })
            .unwrap();

        let report = session.check(&session.require_current().unwrap()).unwrap();
        assert_eq!(report.uncovered, vec!["forgotten.rs"]);
        assert!(!report.passed());
    }

    #[test]
    fn before_wins_over_after_when_both_are_given() {
        assert_eq!(
            position_from(Some("x".into()), Some("y".into())),
            Position::Before("x".into())
        );
    }

    #[test]
    fn no_flags_means_append() {
        assert_eq!(position_from(None, None), Position::End);
    }

    #[test]
    fn a_report_only_passes_when_nothing_is_left_open() {
        assert!(CheckReport::default().passed());
        assert!(
            !CheckReport {
                uncovered: vec!["a.rs".into()],
                ..Default::default()
            }
            .passed()
        );
        assert!(
            !CheckReport {
                pending_orphans: 1,
                ..Default::default()
            }
            .passed()
        );
    }
}
