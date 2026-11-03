use std::fmt::{self, Display};

/// The files read, as the pull request will be told about them.
pub struct ReadList<'a>(pub &'a [String]);

impl Display for ReadList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} current file(s) marked as read:", self.0.len())?;
        for path in self.0 {
            writeln!(f, "  {path}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_lists_each_file_the_pull_request_will_be_told_was_read() {
        // The publishing skill reads this to know what a sync would tick.
        let text = ReadList(&["src/a.rs".into()]).to_string();
        assert!(text.contains("1 current file(s)"), "{text}");
        assert!(text.contains("  src/a.rs"), "{text}");
    }
}
