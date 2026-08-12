//! The files under review, as farol resolved them.

use std::fmt::{self, Display};

use crate::diff::domain::Scope;

/// The files under review, as farol resolved them.
pub struct ScopeReport<'a>(pub &'a Scope);

impl Display for ScopeReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope = self.0;
        let separator = if scope.merge_base { "..." } else { ".." };
        writeln!(
            f,
            "review scope for {}{separator}{}",
            scope.base_ref, scope.head_ref
        )?;
        if scope.dirty {
            writeln!(f, "including uncommitted changes")?;
        }
        writeln!(f, "{} files\n", scope.files.len())?;

        for file in &scope.files {
            let rename = file
                .old_path
                .as_ref()
                .map(|p| format!(" (was {p})"))
                .unwrap_or_default();
            writeln!(
                f,
                "  {:<10} +{:<5} -{:<5} {}{rename}",
                file.status.label(),
                file.additions,
                file.deletions,
                file.path,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::domain::{FileChange, FileStatus};

    fn change(path: &str, status: FileStatus, old_path: Option<&str>) -> FileChange {
        FileChange {
            content_hash: format!("hash-of-{path}"),
            path: path.into(),
            old_path: old_path.map(Into::into),
            status,
            additions: 12,
            deletions: 3,
        }
    }

    fn scope(files: Vec<FileChange>) -> Scope {
        Scope {
            branch: "feature/x".into(),
            base_ref: "main".into(),
            head_ref: "feature/x".into(),
            base_sha: "aaa".into(),
            head_sha: "bbb".into(),
            merge_base: true,
            dirty: false,
            files,
        }
    }

    #[test]
    fn the_window_is_named_the_way_git_would_name_it() {
        // Three dots is a merge base, two is a direct comparison. Printing the
        // wrong one would misdescribe what the reviewer is about to read.
        let out = ScopeReport(&scope(vec![])).to_string();
        assert!(out.contains("review scope for main...feature/x"), "{out}");

        let mut direct = scope(vec![]);
        direct.merge_base = false;
        let out = ScopeReport(&direct).to_string();
        assert!(out.contains("review scope for main..feature/x"), "{out}");
    }

    #[test]
    fn every_file_arrives_with_its_status_and_its_churn() {
        let out =
            ScopeReport(&scope(vec![change("src/a.rs", FileStatus::Modified, None)])).to_string();

        assert!(out.contains("1 files"), "{out}");
        assert!(out.contains("modified"), "{out}");
        assert!(out.contains("+12"), "{out}");
        assert!(out.contains("-3"), "{out}");
        assert!(out.contains("src/a.rs"), "{out}");
    }

    #[test]
    fn a_renamed_file_says_where_it_came_from() {
        // Without it the reviewer cannot tell a move from a file appearing out
        // of nowhere.
        let out = ScopeReport(&scope(vec![change(
            "new/a.rs",
            FileStatus::Renamed,
            Some("old/a.rs"),
        )]))
        .to_string();

        assert!(out.contains("new/a.rs (was old/a.rs)"), "{out}");
    }

    #[test]
    fn uncommitted_work_is_announced_because_it_will_not_be_there_tomorrow() {
        let mut dirty = scope(vec![]);
        dirty.dirty = true;

        assert!(
            ScopeReport(&dirty)
                .to_string()
                .contains("including uncommitted changes")
        );
    }

    #[test]
    fn an_empty_window_still_says_so_rather_than_printing_nothing() {
        let out = ScopeReport(&scope(vec![])).to_string();

        assert!(out.contains("0 files"), "{out}");
        assert!(!out.contains("including uncommitted"), "{out}");
    }
}
