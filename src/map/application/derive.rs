//! Producing the version of the map that belongs to the current commit.
//!
//! Deriving rather than regenerating exists for one reason: whoever is reading
//! may be halfway through. Rebuilding from scratch reshuffles block boundaries
//! and names, and they lose their place even though the new map is just as
//! good. A typo commit must not cost that.

use crate::diff::domain::DiffSource;
use crate::map::domain::{
    MapRepository, Orphan, OrphanReason, ReviewMap, ShiftOutcome, WORKING, shift_range,
};
use crate::shared::error::Result;

pub struct Derived {
    pub map: ReviewMap,
    /// True when this call created the version rather than finding it.
    pub created: bool,
}

/// Idempotent: creates the version for the current commit if it is missing,
/// returns the existing one otherwise. Either way the caller can read the
/// pending orphans off the map, so a run that died halfway simply picks up.
pub fn derive(source: &dyn DiffSource, repo: &dyn MapRepository) -> Result<Derived> {
    let scope = source.scope()?;
    let target = if scope.dirty {
        WORKING.to_string()
    } else {
        scope.head_sha.clone()
    };

    if let Some(existing) = repo.load_at(&target)? {
        return Ok(Derived {
            map: existing,
            created: false,
        });
    }

    let parent = find_parent(source, repo, &target)?;

    let mut map = match parent {
        Some((parent_sha, parent_map)) => {
            let mut m = parent_map;
            m.parent = Some(parent_sha.clone());
            m.generated_at = target.clone();
            m.branch = scope.branch.clone();
            m.base = scope.base_ref.clone();
            // Orphans belong to the version that produced them; the next one
            // starts with a clean slate. Carrying them forever would pile up a
            // graveyard nobody revisits.
            m.orphans.clear();
            reanchor(&mut m, source, &parent_sha, &target)?;
            m
        }
        None => ReviewMap::new(&scope.branch, &scope.base_ref, &target),
    };

    prune_gone_files(&mut map, source)?;
    repo.save(&map)?;

    // A working map that has been absorbed must not keep being picked as the
    // parent of every future commit.
    if !scope.dirty
        && let Some(p) = &map.parent
        && p == WORKING
    {
        repo.delete(WORKING)?;
    }

    Ok(Derived { map, created: true })
}

/// The map to inherit from: uncommitted work first, then the newest stored
/// commit that is an ancestor of where we are now.
fn find_parent(
    source: &dyn DiffSource,
    repo: &dyn MapRepository,
    target: &str,
) -> Result<Option<(String, ReviewMap)>> {
    if target != WORKING
        && let Some(wip) = repo.load_at(WORKING)?
    {
        return Ok(Some((WORKING.to_string(), wip)));
    }

    let mut best: Option<(u32, String)> = None;
    for sha in repo.stored_shas()? {
        if sha == target || sha == WORKING {
            continue;
        }
        if !source.is_ancestor(&sha)? {
            continue;
        }
        let distance = source.commits_ahead_of(&sha)?;
        if best.as_ref().map(|(d, _)| distance < *d).unwrap_or(true) {
            best = Some((distance, sha));
        }
    }

    match best {
        Some((_, sha)) => Ok(repo.load_at(&sha)?.map(|m| (sha, m))),
        None => Ok(None),
    }
}

/// Move every line note onto the new commit, deactivating the ones whose code
/// was rewritten.
fn reanchor(map: &mut ReviewMap, source: &dyn DiffSource, from: &str, to: &str) -> Result<()> {
    let mut orphans = Vec::new();

    for block in &mut map.blocks {
        for file in &mut block.files {
            if file.line_notes.is_empty() {
                continue;
            }
            let between = source.file_diff_between(from, to, &file.path)?;
            let Some(diff) = between else {
                continue; // file untouched: every note is still exactly right
            };

            let mut kept = Vec::new();
            for note in std::mem::take(&mut file.line_notes) {
                match shift_range(note.from, note.to, &diff.hunks) {
                    ShiftOutcome::Unchanged => kept.push(note),
                    ShiftOutcome::Shifted { from, to } => {
                        kept.push(crate::map::domain::LineNote {
                            from,
                            to,
                            text: note.text,
                        });
                    }
                    ShiftOutcome::Overlapped => {
                        orphans.push(Orphan {
                            block: block.slug.clone(),
                            path: file.path.clone(),
                            old_from: note.from,
                            old_to: note.to,
                            snapshot: snapshot_of(&diff, note.from, note.to),
                            reason: OrphanReason::HunkOverlap,
                            text: note.text,
                        });
                    }
                }
            }
            file.line_notes = kept;
        }
    }

    map.orphans.extend(orphans);
    Ok(())
}

/// The code the note used to cover. This — not the old line numbers — is what
/// identifies the note afterwards: after a refactor, `82-116` may point at a
/// completely different function, and restoring there would place a confident
/// note on unrelated code.
fn snapshot_of(diff: &crate::diff::domain::FileDiff, from: u32, to: u32) -> String {
    let mut lines = Vec::new();
    for hunk in &diff.hunks {
        for line in &hunk.lines {
            if let Some(n) = line.old_number
                && n >= from
                && n <= to
            {
                lines.push(line.content.clone());
            }
        }
    }
    let joined = lines.join("\n");
    if joined.chars().count() > 600 {
        joined.chars().take(600).collect::<String>() + "\n…"
    } else {
        joined
    }
}

/// Files that left the review window cannot stay in the map. Their notes are
/// kept as orphans so the prose can be moved rather than silently lost.
fn prune_gone_files(map: &mut ReviewMap, source: &dyn DiffSource) -> Result<()> {
    let scope = source.scope()?;
    let mut orphans = Vec::new();

    for block in &mut map.blocks {
        let slug = block.slug.clone();
        block.files.retain(|file| {
            if scope.contains(&file.path) {
                return true;
            }
            for note in &file.line_notes {
                orphans.push(Orphan {
                    block: slug.clone(),
                    path: file.path.clone(),
                    old_from: note.from,
                    old_to: note.to,
                    snapshot: String::new(),
                    reason: OrphanReason::FileRemoved,
                    text: note.text.clone(),
                });
            }
            false
        });
    }

    map.skim.retain(|s| scope.contains(&s.path));
    map.orphans.extend(orphans);
    Ok(())
}
