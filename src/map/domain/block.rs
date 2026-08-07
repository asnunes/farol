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
