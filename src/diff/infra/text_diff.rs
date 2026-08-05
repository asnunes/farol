//! Turning a computed diff into hunks the reviewer can read.
//!
//! The comparison itself is git's — `imara-diff`, the engine gitoxide uses, run
//! with the algorithm the repository is configured for. What is left here is
//! grouping the changed ranges with context, which is the shape the screen and
//! the line notes need.

use gix::diff::blob::{Algorithm, Diff, InternedInput, Interner, Token};

use crate::diff::domain::{Hunk, Line, LineKind};

/// Unchanged lines kept around each change, and the distance at which two
/// changes stop being one hunk. Three is what git shows by default.
const CONTEXT: u32 = 3;

/// Hunks, and the churn that goes in the file header.
pub(super) struct Hunks {
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
}

pub(super) fn hunks(algorithm: Algorithm, input: &InternedInput<&[u8]>) -> Hunks {
    let mut diff = Diff::compute(algorithm, input);
    // The same slider heuristic git applies, so a hunk starts where a reader
    // would say it starts rather than at the first line that happens to differ.
    diff.postprocess_lines(input);

    let mut out = Hunks {
        hunks: Vec::new(),
        additions: 0,
        deletions: 0,
    };
    let mut open: Option<Open> = None;

    for change in diff.hunks() {
        let (before, after) = (change.before, change.after);

        // Two changes closer than twice the context share it, so they read as
        // one hunk instead of two with a sliver of untouched code between them.
        let joins = open
            .as_ref()
            .is_some_and(|o| before.start.saturating_sub(o.before_pos) <= 2 * CONTEXT);

        if !joins {
            if let Some(o) = open.take() {
                out.hunks.push(o.close(input));
            }
            open = Some(Open::starting_at(
                before.start.saturating_sub(CONTEXT),
                after.start.saturating_sub(CONTEXT),
            ));
        }

        let o = open.as_mut().expect("a hunk is open by now");
        o.carry_context(input, before.start);

        for &token in &input.before[before.start as usize..before.end as usize] {
            o.push(&input.interner, LineKind::Removed, token);
            out.deletions += 1;
        }
        for &token in &input.after[after.start as usize..after.end as usize] {
            o.push(&input.interner, LineKind::Added, token);
            out.additions += 1;
        }

        o.before_pos = before.end;
        o.after_pos = after.end;
    }

    if let Some(o) = open.take() {
        out.hunks.push(o.close(input));
    }
    out
}

/// A hunk being built, and where each side has got to.
struct Open {
    old_start: u32,
    new_start: u32,
    /// Next line to read on each side, as an index into the token list.
    before_pos: u32,
    after_pos: u32,
    /// Line numbers handed out so far, which is what the notes anchor to.
    old_next: u32,
    new_next: u32,
    lines: Vec<Line>,
}

impl Open {
    fn starting_at(before: u32, after: u32) -> Self {
        Self {
            old_start: before + 1,
            new_start: after + 1,
            before_pos: before,
            after_pos: after,
            old_next: before + 1,
            new_next: after + 1,
            lines: Vec::new(),
        }
    }

    /// Unchanged lines between where we are and `until`, which both sides share.
    fn carry_context(&mut self, input: &InternedInput<&[u8]>, until: u32) {
        for &token in &input.before[self.before_pos as usize..until as usize] {
            self.push(&input.interner, LineKind::Context, token);
        }
        let carried = until - self.before_pos;
        self.before_pos = until;
        self.after_pos += carried;
    }

    fn push(&mut self, interner: &Interner<&[u8]>, kind: LineKind, token: Token) {
        let old_number = matches!(kind, LineKind::Context | LineKind::Removed).then(|| {
            self.old_next += 1;
            self.old_next - 1
        });
        let new_number = matches!(kind, LineKind::Context | LineKind::Added).then(|| {
            self.new_next += 1;
            self.new_next - 1
        });
        self.lines.push(Line {
            kind,
            old_number,
            new_number,
            // The line separator is never part of the line: whether it
            // survived interning depends on the source, and a note anchored to
            // a line should not.
            content: String::from_utf8_lossy(interner[token])
                .trim_end_matches('\n')
                .to_string(),
        });
    }

    /// Close with trailing context, clamped to the end of the file.
    fn close(mut self, input: &InternedInput<&[u8]>) -> Hunk {
        let end = (self.before_pos + CONTEXT).min(input.before.len() as u32);
        self.carry_context(input, end);

        Hunk {
            old_lines: self.old_next - self.old_start,
            new_lines: self.new_next - self.new_start,
            old_start: self.old_start,
            new_start: self.new_start,
            lines: self.lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix::diff::blob::sources::byte_lines;

    fn diff(old: &str, new: &str) -> Hunks {
        let input = InternedInput::new(byte_lines(old.as_bytes()), byte_lines(new.as_bytes()));
        hunks(Algorithm::Histogram, &input)
    }

    #[test]
    fn a_hunk_carries_the_line_numbers_the_notes_will_anchor_to() {
        let old = "one\ntwo\nthree\nfour\nfive\n";
        let new = "one\ntwo\nCHANGED\nfour\nfive\n";

        let out = diff(old, new);

        assert_eq!((out.additions, out.deletions), (1, 1));
        assert_eq!(out.hunks.len(), 1);
        let hunk = &out.hunks[0];
        assert_eq!((hunk.old_start, hunk.old_lines), (1, 5));
        assert_eq!((hunk.new_start, hunk.new_lines), (1, 5));

        let removed = hunk
            .lines
            .iter()
            .find(|l| l.kind == LineKind::Removed)
            .unwrap();
        assert_eq!(removed.old_number, Some(3));
        assert_eq!(removed.new_number, None);

        let added = hunk
            .lines
            .iter()
            .find(|l| l.kind == LineKind::Added)
            .unwrap();
        assert_eq!(added.new_number, Some(3));
        assert_eq!(added.old_number, None);
        assert_eq!(added.content, "CHANGED");
    }

    #[test]
    fn distant_edits_land_in_separate_hunks() {
        let old: String = (1..=40).map(|i| format!("line {i}\n")).collect();
        let new = old
            .replace("line 2\n", "CHANGED 2\n")
            .replace("line 38\n", "CHANGED 38\n");

        let out = diff(&old, &new);

        assert_eq!(out.hunks.len(), 2);
        assert_eq!((out.additions, out.deletions), (2, 2));
    }

    #[test]
    fn edits_close_together_share_one_hunk() {
        // Two changes four lines apart: splitting them would put a sliver of
        // untouched code on its own between two headers.
        let old: String = (1..=40).map(|i| format!("line {i}\n")).collect();
        let new = old
            .replace("line 10\n", "CHANGED 10\n")
            .replace("line 14\n", "CHANGED 14\n");

        let out = diff(&old, &new);

        assert_eq!(out.hunks.len(), 1);
    }

    #[test]
    fn a_change_at_the_top_of_the_file_does_not_reach_for_context_that_is_not_there() {
        let out = diff("one\ntwo\nthree\n", "CHANGED\ntwo\nthree\n");

        let hunk = &out.hunks[0];
        assert_eq!(hunk.old_start, 1, "there is no line 0 to start from");
        assert_eq!(hunk.old_lines, 3);
    }

    #[test]
    fn a_change_at_the_end_of_the_file_stops_at_the_last_line() {
        let out = diff("one\ntwo\nthree\n", "one\ntwo\nCHANGED\n");

        let hunk = &out.hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(
            hunk.old_lines, 3,
            "trailing context cannot run past the end of the file"
        );
    }

    #[test]
    fn an_added_file_is_all_additions_and_no_context() {
        let out = diff("", "one\ntwo\n");

        assert_eq!((out.additions, out.deletions), (2, 0));
        assert!(out.hunks[0].lines.iter().all(|l| l.kind == LineKind::Added));
        assert_eq!(out.hunks[0].new_start, 1);
    }

    /// Hunk headers as `git diff` itself prints them, for the same pair.
    fn git_headers(old: &str, new: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        std::fs::write(&a, old).unwrap();
        std::fs::write(&b, new).unwrap();

        let out = std::process::Command::new("git")
            .args(["diff", "--no-index", "--unified=3", "--no-color"])
            .arg(&a)
            .arg(&b)
            .output()
            .expect("git should run");

        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("@@"))
            .map(|l| l.split("@@").nth(1).unwrap_or("").trim().to_string())
            .collect()
    }

    fn our_headers(old: &str, new: &str) -> Vec<String> {
        let input = InternedInput::new(byte_lines(old.as_bytes()), byte_lines(new.as_bytes()));
        hunks(Algorithm::Myers, &input)
            .hunks
            .iter()
            .map(|h| {
                format!(
                    "-{},{} +{},{}",
                    h.old_start, h.old_lines, h.new_start, h.new_lines
                )
            })
            .collect()
    }

    #[test]
    fn hunk_boundaries_are_the_ones_git_would_draw() {
        // The grouping is ours; the rule it implements is git's. Anything that
        // drifts here moves every line note by the same amount.
        let base: String = (1..=60).map(|i| format!("line {i}\n")).collect();

        let cases: Vec<(String, String)> = vec![
            // one change in the middle
            (base.clone(), base.replace("line 30\n", "CHANGED\n")),
            // two changes far apart: two hunks
            (
                base.clone(),
                base.replace("line 5\n", "A\n").replace("line 50\n", "B\n"),
            ),
            // two changes close together: one hunk
            (
                base.clone(),
                base.replace("line 20\n", "A\n").replace("line 24\n", "B\n"),
            ),
            // exactly at the joining distance
            (
                base.clone(),
                base.replace("line 20\n", "A\n").replace("line 27\n", "B\n"),
            ),
            // first line, where there is no context above
            (base.clone(), base.replace("line 1\n", "FIRST\n")),
            // last line, where there is none below
            (base.clone(), base.replace("line 60\n", "LAST\n")),
            // a pure insertion
            (base.clone(), base.replace("line 30\n", "line 30\nEXTRA\n")),
            // a pure deletion
            (base.clone(), base.replace("line 30\n", "")),
        ];

        for (i, (old, new)) in cases.iter().enumerate() {
            assert_eq!(
                our_headers(old, new),
                git_headers(old, new),
                "case {i} disagrees with git"
            );
        }
    }

    #[test]
    fn an_unchanged_file_has_no_hunks() {
        let out = diff("one\ntwo\n", "one\ntwo\n");
        assert!(out.hunks.is_empty());
        assert_eq!((out.additions, out.deletions), (0, 0));
    }
}
