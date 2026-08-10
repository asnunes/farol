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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proven_path_reads_as_the_string_it_stands_for() {
        // It is printed into error messages and command suggestions, where a
        // wrapper's debug shape would be noise.
        let path = ReviewPath::proven("src/a.rs", 40);

        assert_eq!(path.to_string(), "src/a.rs");
        assert_eq!(path.as_str(), "src/a.rs");
        assert_eq!(<ReviewPath as AsRef<str>>::as_ref(&path), "src/a.rs");
    }

    #[test]
    fn two_paths_are_the_same_when_they_name_the_same_file_at_the_same_length() {
        assert_eq!(
            ReviewPath::proven("a.rs", 10),
            ReviewPath::proven("a.rs", 10)
        );
        assert_ne!(
            ReviewPath::proven("a.rs", 10),
            ReviewPath::proven("b.rs", 10)
        );
    }
}
