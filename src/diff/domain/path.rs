use crate::shared::error::Result;

/// A path proven to be part of the review, carrying how long the file is.
///
/// The only way to build one is to ask a [`DiffSource`](super::DiffSource), so a
/// use case that takes this cannot be handed a path the reviewer will never
/// see. That is the difference between a check a caller might forget and one it
/// cannot express.
///
/// The line count rides along because it is known at the same moment and is
/// what line ranges are checked against — otherwise every range check would
/// have to reach back for the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewPath {
    path: String,
    lines: u32,
}

impl ReviewPath {
    /// Only [`DiffSource::review_path`](super::DiffSource::review_path) should
    /// call this; it is the proof that the path was checked.
    pub(crate) fn proven(path: impl Into<String>, lines: u32) -> Self {
        Self {
            path: path.into(),
            lines,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn lines(&self) -> u32 {
        self.lines
    }
}

impl std::fmt::Display for ReviewPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.path)
    }
}

impl AsRef<str> for ReviewPath {
    fn as_ref(&self) -> &str {
        &self.path
    }
}

/// Resolve several at once, failing on the first that is not under review.
pub fn all(source: &dyn super::DiffSource, raw: &[String]) -> Result<Vec<ReviewPath>> {
    raw.iter().map(|p| source.review_path(p)).collect()
}
