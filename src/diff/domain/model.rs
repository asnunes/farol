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
    pub fn header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_lines, self.new_start, self.new_lines
        )
    }

    /// Net line growth this hunk introduces.
    pub fn delta(&self) -> i64 {
        self.new_lines as i64 - self.old_lines as i64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
    /// Hash of the file contents *after* the change. Viewed-state invalidation
    /// keys on this and not on the diff text, so a rebase that only shifts
    /// context does not reopen the whole branch.
    pub new_content_hash: String,
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
        self.files.iter().any(|f| f.path == path)
    }

    pub fn paths(&self) -> Vec<&str> {
        self.files.iter().map(|f| f.path.as_str()).collect()
    }

    /// Reject a path that is not under review, carrying the near misses with
    /// it. Built here so every caller rejects the same way.
    pub fn reject(&self, path: &str) -> crate::shared::error::Error {
        crate::shared::error::Error::PathOutOfScope {
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
