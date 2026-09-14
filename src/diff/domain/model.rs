use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl FileStatus {
    pub fn label(&self) -> &'static str {
        match self {
            FileStatus::Added => "added",
            FileStatus::Modified => "modified",
            FileStatus::Deleted => "deleted",
            FileStatus::Renamed => "renamed",
        }
    }
}

/// One entry of the review scope: a file the reviewer is expected to look at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    /// Set only when the file was renamed.
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
    /// git's id for the content under review, which is what a mark of `read`
    /// is anchored to. A deleted file carries the content that went away.
    pub content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub kind: LineKind,
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
    pub content: String,
}

/// A contiguous changed region. `old_*` describes the pre-image side, which is
/// what line-note shifting reasons about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<Line>,
}

impl Hunk {
    /// Net line growth this hunk introduces.
    pub fn delta(&self) -> i64 {
        self.new_lines as i64 - self.old_lines as i64
    }

    /// Whether this hunk shows the given line of the file as it now reads.
    ///
    /// Context counts. A hunk prints the lines around what changed, and those
    /// are as much a part of the diff as the changed ones — they are exactly
    /// where a reader asks why the change was needed.
    pub fn shows(&self, line: u32) -> bool {
        line >= self.new_start && line < self.new_start + self.new_lines
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub hunks: Vec<Hunk>,
    /// git refuses to diff this file — binary content, or `-diff` in
    /// `.gitattributes`. There are no hunks, and the screen says so instead of
    /// rendering decoded bytes as if they were code.
    pub binary: bool,
    pub additions: u32,
    pub deletions: u32,
    /// How long the file is after the change. The screen needs it to know
    /// whether there is anything left below the last hunk to open.
    pub line_count: u32,
    /// Hash of the file contents *after* the change. Viewed-state invalidation
    /// keys on this and not on the diff text, so a rebase that only shifts
    /// context does not reopen the whole branch.
    pub new_content_hash: String,
}

impl FileDiff {
    /// Whether every line of a span appears in the diff.
    ///
    /// A comment can only be left where the diff reaches. GitHub refuses a
    /// review comment on a line it is not showing, and farol has to refuse it
    /// first — otherwise the comment is written, kept, and rejected later by a
    /// machine that cannot explain itself.
    ///
    /// The whole span, not just its ends: a range that starts in one hunk and
    /// finishes in the next spans a gap the diff never printed.
    pub fn shows(&self, from: u32, to: u32) -> bool {
        (from..=to).all(|line| self.hunks.iter().any(|hunk| hunk.shows(line)))
    }
}

/// The resolved review window plus the files inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub branch: String,
    pub base_ref: String,
    pub head_ref: String,
    /// Merge-base with the base ref, unless `--direct` was asked for.
    pub base_sha: String,
    pub head_sha: String,
    pub merge_base: bool,
    pub dirty: bool,
    pub files: Vec<FileChange>,
}

impl Scope {
    pub fn contains(&self, path: &str) -> bool {
        self.change(path).is_some()
    }

    /// The change git reports for a path, or nothing when the path is outside
    /// the comparison. A map outlives the comparison it was written against —
    /// once the base moves, some of its files are simply no longer here.
    pub fn change(&self, path: &str) -> Option<&FileChange> {
        self.files.iter().find(|f| f.path == path)
    }

    /// Reject a path that is not under review, carrying the near misses with
    /// it. Built here so every caller rejects the same way.
    pub fn reject(&self, path: &str) -> super::ScopeError {
        super::ScopeError::PathOutOfScope {
            path: path.to_string(),
            similar: self.similar_paths(path),
        }
    }

    /// Paths that look like `candidate`, for "did you mean" on a rejected path.
    /// An LLM writing a map hallucinates paths more than anything else, so the
    /// suggestion is what turns a rejection into a self-correction.
    pub fn similar_paths(&self, candidate: &str) -> Vec<String> {
        let needle = candidate
            .rsplit('/')
            .next()
            .unwrap_or(candidate)
            .to_lowercase();
        let mut hits: Vec<(usize, String)> = self
            .files
            .iter()
            .filter_map(|f| {
                let name = f.path.rsplit('/').next().unwrap_or(&f.path).to_lowercase();
                if name == needle {
                    Some((0, f.path.clone()))
                } else if name.contains(&needle) || needle.contains(&name) {
                    Some((1, f.path.clone()))
                } else {
                    None
                }
            })
            .collect();
        hits.sort();
        hits.into_iter().take(5).map(|(_, p)| p).collect()
    }
}

#[cfg(test)]
mod tests {
    /// A diff that shows lines 10 to 14 and lines 40 to 42, and nothing else.
    fn two_hunks() -> FileDiff {
        FileDiff {
            path: "a.rs".into(),
            old_path: None,
            status: FileStatus::Modified,
            line_count: 100,
            hunks: vec![
                Hunk {
                    old_start: 10,
                    old_lines: 5,
                    new_start: 10,
                    new_lines: 5,
                    lines: vec![],
                },
                Hunk {
                    old_start: 40,
                    old_lines: 3,
                    new_start: 40,
                    new_lines: 3,
                    lines: vec![],
                },
            ],
            binary: false,
            additions: 0,
            deletions: 0,
            new_content_hash: String::new(),
        }
    }

    #[test]
    fn a_span_inside_one_hunk_is_shown() {
        assert!(two_hunks().shows(11, 13));
    }

    #[test]
    fn the_first_and_last_lines_of_a_hunk_count() {
        // Off by one here is the difference between a comment landing and being
        // refused by GitHub after the reviewer already wrote it.
        let diff = two_hunks();
        assert!(diff.shows(10, 10), "the first line the hunk prints");
        assert!(diff.shows(14, 14), "the last one");
        assert!(!diff.shows(9, 9), "one above");
        assert!(!diff.shows(15, 15), "one below");
    }

    #[test]
    fn a_line_between_two_hunks_is_not_shown() {
        // The diff jumps from 14 to 40. Nothing printed line 27.
        assert!(!two_hunks().shows(27, 27));
    }

    #[test]
    fn a_span_that_reaches_across_the_gap_is_not_shown() {
        // Both ends are in the diff and the middle is not, so a comment on it
        // would cover code the reader never saw.
        assert!(!two_hunks().shows(14, 40));
    }

    #[test]
    fn a_binary_file_shows_nothing() {
        let mut diff = two_hunks();
        diff.hunks.clear();
        diff.binary = true;

        assert!(!diff.shows(1, 1));
    }

    use super::*;

    fn scope_over(paths: &[&str]) -> Scope {
        Scope {
            branch: "b".into(),
            base_ref: "main".into(),
            head_ref: "b".into(),
            base_sha: "x".into(),
            head_sha: "y".into(),
            merge_base: true,
            dirty: false,
            files: paths
                .iter()
                .map(|p| FileChange {
                    path: p.to_string(),
                    old_path: None,
                    status: FileStatus::Modified,
                    additions: 0,
                    deletions: 0,
                    content_hash: format!("hash-of-{p}"),
                })
                .collect(),
        }
    }

    #[test]
    fn a_near_miss_path_is_offered_back_best_match_first() {
        // The reader of this suggestion is the session writing the map, and a
        // hallucinated path is the mistake it makes most; the suggestion is
        // what turns a rejection into a self-correction.
        let scope = scope_over(&["src/store/db.rs", "src/io/db_test.rs", "unrelated.py"]);

        let hits = scope.similar_paths("src/stores/db.rs");
        assert_eq!(
            hits.first().map(String::as_str),
            Some("src/store/db.rs"),
            "same file name, wrong directory, is the closest kind of miss"
        );
    }

    #[test]
    fn a_path_that_looks_like_nothing_under_review_gets_no_suggestion() {
        // Better silence than sending the author off to another wrong path.
        let scope = scope_over(&["src/store/db.rs"]);
        assert!(scope.similar_paths("nothing_like_it.py").is_empty());
    }
}
