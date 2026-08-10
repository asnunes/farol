//! A block, the files it holds, and the notes pinned inside them.
//!
//! One file rather than three: a change to how a block carries its files
//! changes all of them together.

use serde::{Deserialize, Serialize};

use super::range::LineRange;
use super::slug::Slug;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub slug: Slug,
    pub title: String,
    pub context: String,
    #[serde(default)]
    pub files: Vec<BlockFile>,
}

impl Block {
    pub fn file(&self, path: &str) -> Option<&BlockFile> {
        self.files.iter().find(|f| f.path == path)
    }

    pub fn file_mut(&mut self, path: &str) -> Option<&mut BlockFile> {
        self.files.iter_mut().find(|f| f.path == path)
    }

    /// Used to name the alternatives when a path is not in the block.
    pub(crate) fn paths(&self) -> Vec<String> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockFile {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_notes: Vec<LineNote>,
}

impl BlockFile {
    pub fn new(path: impl Into<String>, note: Option<String>) -> Self {
        Self {
            path: path.into(),
            note,
            line_notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineNote {
    #[serde(flatten)]
    pub range: LineRange,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> Block {
        Block {
            slug: crate::testing::slug("core"),
            title: "t".into(),
            context: "c".into(),
            files: vec![
                BlockFile::new("a.rs", None),
                BlockFile::new("b.rs", Some("why b".into())),
            ],
        }
    }

    #[test]
    fn a_file_is_found_by_the_path_it_was_filed_under() {
        let b = block();

        assert_eq!(b.file("b.rs").unwrap().note.as_deref(), Some("why b"));
        assert!(b.file("nowhere.rs").is_none());
    }

    #[test]
    fn a_file_can_be_reached_for_changing_too() {
        let mut b = block();

        b.file_mut("a.rs").unwrap().note = Some("added later".into());

        assert_eq!(b.file("a.rs").unwrap().note.as_deref(), Some("added later"));
        assert!(b.file_mut("nowhere.rs").is_none());
    }

    #[test]
    fn the_paths_come_back_in_reading_order() {
        // They are printed back when a path is not in the block, so the author
        // can see what is.
        assert_eq!(block().paths(), vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn a_new_file_starts_with_no_line_notes() {
        let f = BlockFile::new("a.rs", Some("a note".into()));

        assert_eq!(f.path, "a.rs");
        assert_eq!(f.note.as_deref(), Some("a note"));
        assert!(f.line_notes.is_empty());
    }
}
