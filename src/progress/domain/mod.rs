use serde::{Deserialize, Serialize};

use crate::shared::error::Result;

pub const PROGRESS_VERSION: u32 = 1;

/// A file the reviewer marked as read, pinned to the content it had at the time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewedFile {
    pub path: String,
    /// Hash of the file contents *after* the change — deliberately not the diff
    /// text. Hashing the diff would reopen the whole branch on a rebase that
    /// only shifted context lines.
    pub content_hash: String,
    pub at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub viewed: Vec<ViewedFile>,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            version: PROGRESS_VERSION,
            viewed: Vec::new(),
        }
    }

    /// Read *and* still current. A file whose hash moved on is not read anymore,
    /// which is what reopens it after the author changes it.
    pub fn is_current(&self, path: &str, content_hash: &str) -> bool {
        self.viewed
            .iter()
            .any(|v| v.path == path && v.content_hash == content_hash)
    }

    pub fn mark(
        &mut self,
        path: impl Into<String>,
        content_hash: impl Into<String>,
        at: impl Into<String>,
    ) {
        let path = path.into();
        self.viewed.retain(|v| v.path != path);
        self.viewed.push(ViewedFile {
            path,
            content_hash: content_hash.into(),
            at: at.into(),
        });
    }

    pub fn unmark(&mut self, path: &str) {
        self.viewed.retain(|v| v.path != path);
    }

    /// How many of `paths` are read at their current hash.
    pub fn count_current<'a>(&self, paths: impl Iterator<Item = (&'a str, &'a str)>) -> usize {
        paths.filter(|(p, h)| self.is_current(p, h)).count()
    }
}

pub trait ProgressRepository: Send + Sync {
    fn load(&self) -> Result<Progress>;
    fn save(&self, progress: &Progress) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marking_then_asking_with_the_same_hash_says_read() {
        let mut p = Progress::new();
        p.mark("a.rs", "hash-1", "now");
        assert!(p.is_current("a.rs", "hash-1"));
    }

    #[test]
    fn changed_content_reopens_the_file() {
        let mut p = Progress::new();
        p.mark("a.rs", "hash-1", "now");
        assert!(!p.is_current("a.rs", "hash-2"));
    }

    #[test]
    fn a_rebase_that_only_moves_context_keeps_the_file_read() {
        // Same post-change content, different diff text: the hash is of the
        // content, so nothing reopens.
        let mut p = Progress::new();
        p.mark("a.rs", "content-hash", "now");
        assert!(p.is_current("a.rs", "content-hash"));
    }

    #[test]
    fn marking_twice_does_not_duplicate() {
        let mut p = Progress::new();
        p.mark("a.rs", "hash-1", "then");
        p.mark("a.rs", "hash-2", "now");
        assert_eq!(p.viewed.len(), 1);
        assert_eq!(p.viewed[0].content_hash, "hash-2");
    }

    #[test]
    fn unmark_removes_it() {
        let mut p = Progress::new();
        p.mark("a.rs", "hash-1", "now");
        p.unmark("a.rs");
        assert!(!p.is_current("a.rs", "hash-1"));
    }

    #[test]
    fn counting_ignores_files_whose_hash_moved_on() {
        let mut p = Progress::new();
        p.mark("a.rs", "h1", "now");
        p.mark("b.rs", "old", "now");
        let files = [("a.rs", "h1"), ("b.rs", "new"), ("c.rs", "h3")];
        assert_eq!(p.count_current(files.iter().copied()), 1);
    }
}
