//! Turning two versions of a file into hunks the reviewer can read.
//!
//! Nothing here touches git: it works on bytes, and the only reason it lives in
//! `infra` is that `similar` is a choice of library like `gix` is.

use crate::diff::domain::{FileDiff, FileStatus, Hunk, Line, LineKind};

const CONTEXT: usize = 3;

pub(super) fn build_file_diff(
    path: &str,
    old_path: Option<String>,
    status: FileStatus,
    old: &[u8],
    new: &[u8],
    new_content_hash: String,
) -> FileDiff {
    let old_text = String::from_utf8_lossy(old).into_owned();
    let new_text = String::from_utf8_lossy(new).into_owned();
    let diff = similar::TextDiff::from_lines(&old_text, &new_text);

    let mut hunks = Vec::new();
    let mut additions = 0u32;
    let mut deletions = 0u32;

    for group in diff.grouped_ops(CONTEXT) {
        let mut lines = Vec::new();
        let mut old_start = 0u32;
        let mut new_start = 0u32;
        let mut old_lines = 0u32;
        let mut new_lines = 0u32;
        let mut first = true;

        for op in &group {
            for change in diff.iter_changes(op) {
                let old_number = change.old_index().map(|i| i as u32 + 1);
                let new_number = change.new_index().map(|i| i as u32 + 1);
                if first {
                    old_start = old_number.unwrap_or(1);
                    new_start = new_number.unwrap_or(1);
                    first = false;
                }
                let kind = match change.tag() {
                    similar::ChangeTag::Insert => {
                        additions += 1;
                        new_lines += 1;
                        LineKind::Added
                    }
                    similar::ChangeTag::Delete => {
                        deletions += 1;
                        old_lines += 1;
                        LineKind::Removed
                    }
                    similar::ChangeTag::Equal => {
                        old_lines += 1;
                        new_lines += 1;
                        LineKind::Context
                    }
                };
                lines.push(Line {
                    kind,
                    old_number,
                    new_number,
                    content: change.value().trim_end_matches('\n').to_string(),
                });
            }
        }

        hunks.push(Hunk {
            old_start,
            old_lines,
            new_start,
            new_lines,
            lines,
        });
    }

    FileDiff {
        path: path.to_string(),
        old_path,
        status,
        hunks,
        additions,
        deletions,
        new_content_hash,
    }
}

pub(super) fn count_changes(old: &[u8], new: &[u8]) -> (u32, u32) {
    let old = String::from_utf8_lossy(old);
    let new = String::from_utf8_lossy(new);
    let diff = similar::TextDiff::from_lines(old.as_ref(), new.as_ref());
    let mut adds = 0;
    let mut dels = 0;
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Insert => adds += 1,
            similar::ChangeTag::Delete => dels += 1,
            similar::ChangeTag::Equal => {}
        }
    }
    (adds, dels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diff_carries_hunks_with_the_line_numbers_the_notes_will_use() {
        let old = "one\ntwo\nthree\nfour\nfive\n";
        let new = "one\ntwo\nCHANGED\nfour\nfive\n";
        let diff = build_file_diff(
            "a.rs",
            None,
            FileStatus::Modified,
            old.as_bytes(),
            new.as_bytes(),
            String::new(),
        );

        assert_eq!((diff.additions, diff.deletions), (1, 1));
        assert_eq!(diff.hunks.len(), 1);

        let changed: Vec<_> = diff.hunks[0]
            .lines
            .iter()
            .filter(|l| l.kind != LineKind::Context)
            .collect();
        assert_eq!(changed.len(), 2);
        assert_eq!(changed[0].content, "three");
        assert_eq!(changed[0].old_number, Some(3));
        assert_eq!(changed[1].content, "CHANGED");
        assert_eq!(changed[1].new_number, Some(3));
    }

    #[test]
    fn distant_edits_land_in_separate_hunks() {
        // Line notes shift per hunk, so this split is not cosmetic.
        let old: String = (1..=40).map(|i| format!("line {i}\n")).collect();
        let new = old
            .replace("line 2\n", "CHANGED 2\n")
            .replace("line 38\n", "CHANGED 38\n");
        let diff = build_file_diff(
            "a.rs",
            None,
            FileStatus::Modified,
            old.as_bytes(),
            new.as_bytes(),
            String::new(),
        );
        assert_eq!(diff.hunks.len(), 2);
    }
}
