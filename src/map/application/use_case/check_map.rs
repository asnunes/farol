use crate::diff::application::ReviewScope;
use crate::error::Result;
use crate::map::application::MapVersions;

/// About how many files a block opens before it stops choosing: its core,
/// rarely more. A guide the check mentions, never a limit — a pull request
/// has as many cores as it has blocks.
pub const FOCUS_PER_BLOCK: usize = 2;

/// What the self-test found.
#[derive(Debug, Default)]
pub struct CheckReport {
    pub uncovered: Vec<String>,
    pub pending_orphans: usize,
    pub commits_behind: u32,
    /// Blocks that open nothing, or a lot. Hints, not failures: the map is
    /// still readable, it is just showing too little or too much.
    pub focus_hints: Vec<String>,
}

impl CheckReport {
    pub fn passed(&self) -> bool {
        self.uncovered.is_empty() && self.pending_orphans == 0
    }
}

/// Verify the map covers the review and leaves nothing undecided.
///
/// It exists so "every file shows up somewhere" does not depend on the model
/// remembering the rule: a file nobody assigned is not merely undocumented, it
/// is invisible, because the sidebar is built from the map.
#[derive(Clone)]
pub struct CheckMap {
    versions: MapVersions,
    scope: ReviewScope,
}

impl CheckMap {
    pub fn new(versions: MapVersions, scope: ReviewScope) -> Self {
        Self { versions, scope }
    }

    pub fn execute(&self) -> Result<CheckReport> {
        let map = self.versions.require_current()?;
        let covered = map.covered_paths();
        Ok(CheckReport {
            uncovered: self
                .scope
                .get()?
                .files
                .iter()
                .map(|f| f.path.clone())
                .filter(|p| !covered.contains(p))
                .collect(),
            pending_orphans: map.orphans().len(),
            commits_behind: self.versions.behind(&map),
            focus_hints: focus_hints(&map),
        })
    }
}

/// Every block shows its core, and little more. A block that opens nothing
/// is either missing its core or is not a block at all; one that opens many
/// has stopped choosing.
fn focus_hints(map: &crate::map::domain::ReviewMap) -> Vec<String> {
    map.blocks()
        .iter()
        .filter_map(|b| {
            let open = b.files.iter().filter(|f| f.focus.is_some()).count();
            match open {
                0 => Some(format!("block '{}' opens no file", b.slug)),
                n if n > FOCUS_PER_BLOCK => Some(format!("block '{}' opens {n} files", b.slug)),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::MapError;
    use crate::testing::{slug, use_case_setup};

    #[test]
    fn check_fails_on_a_file_nobody_assigned() {
        let paths = ["a.rs", "forgotten.rs"];
        let (svc, scope) = use_case_setup(&paths);
        svc.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", crate::map::domain::Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)
            })
            .unwrap();

        let report = CheckMap::new(svc.versions, scope).execute().unwrap();

        assert_eq!(report.uncovered, vec!["forgotten.rs"]);
        assert!(!report.passed());
    }

    #[test]
    fn a_map_that_covers_every_file_and_leaves_no_orphan_passes() {
        let paths = ["a.rs", "b.rs"];
        let (svc, scope) = use_case_setup(&paths);
        svc.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", crate::map::domain::Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)?;
                map.add_file(&slug("core"), "b.rs", None, None)
            })
            .unwrap();

        let report = CheckMap::new(svc.versions, scope).execute().unwrap();

        assert!(report.uncovered.is_empty());
        assert_eq!(report.pending_orphans, 0);
        assert!(report.passed());
    }

    #[test]
    fn an_orphan_nobody_decided_about_holds_the_check_open() {
        let (svc, scope) = use_case_setup(&["a.rs"]);
        svc.editor
            .edit(|map| {
                map.add_block(&slug("core"), "t", "c", crate::map::domain::Position::End)?;
                map.add_file(&slug("core"), "a.rs", None, None)?;
                map.add_line_note(
                    &slug("core"),
                    "a.rs",
                    crate::testing::range(1, 2),
                    "undecided",
                )?;
                // Deactivated the way a derivation would leave it.
                map.reanchor_notes(|_, _| {
                    Ok(crate::map::domain::NoteFate::Orphan {
                        snapshot: String::new(),
                        reason: crate::map::domain::OrphanReason::HunkOverlap,
                    })
                })?;
                Ok::<_, MapError>(())
            })
            .unwrap();

        let report = CheckMap::new(svc.versions, scope).execute().unwrap();

        assert!(report.uncovered.is_empty());
        assert_eq!(report.pending_orphans, 1);
        assert!(!report.passed());
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

    #[test]
    fn being_behind_is_reported_but_does_not_fail_the_check() {
        // Staleness is something to know, not something that makes the map
        // wrong: the reviewer can keep reading while the branch moves.
        assert!(
            CheckReport {
                commits_behind: 4,
                ..Default::default()
            }
            .passed()
        );
    }

    #[test]
    fn a_block_that_opens_nothing_or_many_files_is_a_hint_and_not_a_failure() {
        let paths = ["a.rs", "b.rs", "c.rs", "d.rs"];
        let (svc, scope) = use_case_setup(&paths);
        svc.editor
            .edit(|map| {
                for (block, path, core) in [
                    ("wide", "a.rs", true),
                    ("wide", "b.rs", true),
                    ("wide", "c.rs", true),
                    ("bare", "d.rs", false),
                ] {
                    if map.block(&slug(block)).is_none() {
                        map.add_block(&slug(block), "t", "c", crate::map::domain::Position::End)?;
                    }
                    map.add_file(&slug(block), path, None, None)?;
                    if core {
                        map.focus(&slug(block), path, Some("why".into()))?;
                    }
                }
                Ok::<_, MapError>(())
            })
            .unwrap();

        let report = CheckMap::new(svc.versions, scope).execute().unwrap();

        assert_eq!(
            report.focus_hints,
            vec!["block 'wide' opens 3 files", "block 'bare' opens no file"]
        );
        assert!(
            report.passed(),
            "a hint about what is open never blocks the skill"
        );
    }
}
