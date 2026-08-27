use crate::diff::application::ReviewScope;
use crate::error::Result;

/// A stretch of one file, for the reader opening what the diff did not print.
///
/// The window is the only thing checked here: what a reader may look at is what
/// the review covers, and how much of it they asked for is between them and the
/// end of the file.
#[derive(Clone)]
pub struct GetFileLines {
    scope: ReviewScope,
}

impl GetFileLines {
    pub fn new(scope: ReviewScope) -> Self {
        Self { scope }
    }

    pub fn execute(&self, path: &str, from: u32, to: u32) -> Result<Vec<String>> {
        self.scope.lines(path, from, to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeDiffSource;
    use std::sync::Arc;

    fn on(paths: &[&str], lines: u32) -> GetFileLines {
        GetFileLines::new(ReviewScope::new(Arc::new(
            FakeDiffSource::with_paths(paths).with_line_count(paths[0], lines),
        )))
    }

    #[test]
    fn the_range_that_was_asked_for_is_the_range_that_comes_back() {
        let lines = on(&["a.rs"], 200).execute("a.rs", 40, 43).unwrap();

        assert_eq!(lines, ["line 40", "line 41", "line 42", "line 43"]);
    }

    #[test]
    fn a_range_running_off_the_end_stops_at_the_end() {
        // The reader opening the last gap asks for twenty lines and the file
        // has three left. Trimmed, not refused: they asked to see what is
        // there, and what is there is three lines.
        let lines = on(&["a.rs"], 42).execute("a.rs", 40, 60).unwrap();

        assert_eq!(lines, ["line 40", "line 41", "line 42"]);
    }

    #[test]
    fn a_range_past_the_end_comes_back_empty_rather_than_failing() {
        let lines = on(&["a.rs"], 42).execute("a.rs", 90, 110).unwrap();

        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn a_file_outside_the_review_is_refused() {
        // Same door as the diff route: the path arrives in a query string, and
        // without this a crafted one would read any file in the repository.
        let err = on(&["a.rs"], 200)
            .execute("../../etc/passwd", 1, 20)
            .unwrap_err();

        assert!(err.to_string().contains("passwd"), "{err}");
    }
}
