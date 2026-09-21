mod use_case;

pub use use_case::*;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use crate::comments::domain::{Comment, CommentError, CommentStore, Found};
use crate::diff::application::FileDiffs;
use crate::diff::domain::{FileDiff, Scope, Side};
use crate::error::Result;
use crate::map::application::GetScope;
use crate::map::domain::LineRange;

/// Everything that can be done to the reviewer's comments.
#[derive(Clone)]
pub struct Comments {
    store: Arc<dyn CommentStore>,
    scope: GetScope,
    diffs: FileDiffs,
}

impl Comments {
    pub fn new(store: Arc<dyn CommentStore>, scope: GetScope, diffs: FileDiffs) -> Self {
        Self {
            store,
            scope,
            diffs,
        }
    }

    /// Everything still waiting for an answer, and the files that could not be
    /// read at all.
    ///
    /// Held to the same rule `add` writes under: a comment exists where the
    /// diff reaches it, and nothing keeps that true afterwards. The branch
    /// moves, the line it sat on goes away, and what is left is counted in the
    /// margin beside the file and drawn on no row — because there is no row
    /// left to draw it on. A question the reader is told about, cannot read,
    /// and cannot close is worse than one that is simply gone.
    ///
    /// Checked on the read rather than in each caller, because this is the one
    /// the browser and the terminal both come through: the count in the sidebar
    /// and the list in `farol comment list` are the same list or they are two
    /// answers to one question.
    pub fn all(&self) -> Result<Found> {
        let scope = self.scope.execute()?;
        let mut found = self.store.list()?;
        found.comments = reached(&scope, &self.diffs, std::mem::take(&mut found.comments))?.live;
        Ok(found)
    }

    /// Written against the review, not against a path someone typed: a file
    /// outside the window, a span past the end of the file and a span the diff
    /// never printed are all refused here rather than in each caller. The CLI
    /// and the browser reach this by different roads and the rule has to be the
    /// same on both.
    ///
    /// The last of the three is GitHub's rule, applied early: a review comment
    /// can only sit on a line the diff reaches. Refusing at writing time costs
    /// the reviewer one attempt; letting it through costs them the comment,
    /// discovered at the moment they meant to send the review.
    ///
    /// The span is counted on the side it was written on, and the two are
    /// checked differently. The length of the file only bounds the new side —
    /// it is the length of the file as it now reads, and the old side is a file
    /// that no longer exists at that length, or at all. What bounds the old
    /// side is the diff itself, which never prints a line the pre-image did not
    /// have.
    pub fn add(&self, path: &str, side: Side, from: u32, to: u32, body: &str) -> Result<Comment> {
        if body.trim().is_empty() {
            return Err(CommentError::Empty.into());
        }

        let path = self.scope.path(path)?;
        let range = LineRange::new(from, to)?;
        if side == Side::New {
            range.require_within(&path)?;
        }
        if !self.diffs.of(path.as_str())?.shows(side, from, to) {
            return Err(CommentError::OutsideDiff {
                path: path.as_str().to_string(),
                side,
                from,
                to,
            }
            .into());
        }

        let comment = Comment {
            id: fresh_id(),
            path: path.as_str().to_string(),
            side,
            from,
            to,
            body: body.trim().to_string(),
            published: None,
        };
        self.store.save(&comment)?;
        Ok(comment)
    }

    /// Close one, which is to say drop it.
    ///
    /// There is no settled-but-kept state. A comment lives as long as it is
    /// waiting for an answer, and the list is therefore always what is still
    /// open — nothing to filter, nothing to reopen.
    pub fn close(&self, id: &str) -> Result<()> {
        match self.store.close(id)? {
            true => Ok(()),
            false => Err(CommentError::Unknown { id: id.into() }.into()),
        }
    }
}

/// Split comments by whether the diff still reaches the lines they sit on.
///
/// The one place that rule lives. It is asked at three moments — writing a
/// comment, listing what is open, and sending the review — and worked out
/// separately at each is how the list and the screen came to disagree about how
/// many questions a file has.
///
/// A path that left the review window has no diff to ask for; asking would
/// fail rather than answer, so it is decided before the question is put.
///
/// One diff per file, not one per comment: a file with a dozen questions on it
/// would otherwise be diffed a dozen times.
fn reached(scope: &Scope, diffs: &FileDiffs, comments: Vec<Comment>) -> Result<Reached> {
    let mut seen: HashMap<String, Option<FileDiff>> = HashMap::new();
    let mut split = Reached::default();

    for comment in comments {
        let diff = match seen.entry(comment.path.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(match scope.contains(&comment.path) {
                true => Some(diffs.of(&comment.path)?),
                false => None,
            }),
        };

        match diff
            .as_ref()
            .is_some_and(|d| d.shows(comment.side, comment.from, comment.to))
        {
            true => split.live.push(comment),
            false => split.drifted.push(comment),
        }
    }

    Ok(split)
}

/// An id that survives leaving this machine.
///
/// Comments are meant to travel: the reviewer's come back to the author, and
/// two sets get read together. A counter would collide the moment that happens,
/// so this is the clock plus the process, which two machines cannot both
/// produce.
fn fresh_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{now:x}-{:x}", std::process::id())
}

/// Comments the diff still prints, and comments it has stopped printing.
///
/// Both halves are wanted, by different callers: listing keeps the first, and
/// publishing names the second so the reviewer is told which of their questions
/// drifted rather than being refused without one.
#[derive(Debug, Default)]
struct Reached {
    live: Vec<Comment>,
    drifted: Vec<Comment>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::infra::MarkdownComments;
    use crate::diff::application::ReviewScope;
    use crate::diff::domain::Hunk;
    use crate::shared::paths::Store;
    use crate::testing::FakeDiffSource;

    /// Comments over a real folder, because the store is half of what `add`
    /// does, and over a review that holds one 200-line file.
    fn comments() -> (tempfile::TempDir, Comments) {
        over(a_file())
    }

    /// The same review, but with the diff reaching only where the test says.
    /// Undeclared, the fake prints the whole file, which is what every test
    /// that is not about the diff's reach wants.
    fn comments_showing(hunks: Vec<Hunk>) -> (tempfile::TempDir, Comments) {
        over(a_file().showing("src/a.rs", hunks))
    }

    /// The one review the tests are written against.
    fn a_file() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["src/a.rs"]).with_line_count("src/a.rs", 200)
    }

    /// The wiring, which is the same either way: the store on disk, and the
    /// scope and the diffs coming off the one source.
    fn over(source: FakeDiffSource) -> (tempfile::TempDir, Comments) {
        let dir = tempfile::tempdir().unwrap();
        let comments = over_at(dir.path(), source);
        (dir, comments)
    }

    /// The same, onto a folder the caller already has — which is how a test
    /// puts one review's comments in front of a later review's diff.
    fn over_at(dir: &std::path::Path, source: FakeDiffSource) -> Comments {
        let store = Store::new(dir, "feature/x");
        let source = Arc::new(source);
        Comments::new(
            Arc::new(MarkdownComments::new(&store, dir)),
            GetScope::new(ReviewScope::new(source.clone())),
            FileDiffs::new(source),
        )
    }

    fn hunk(start: u32, lines: u32) -> Hunk {
        Hunk {
            old_start: start,
            old_lines: lines,
            new_start: start,
            new_lines: lines,
            lines: vec![],
        }
    }

    #[test]
    fn a_comment_is_written_and_read_back() {
        let (_dir, comments) = comments();
        comments
            .add("src/a.rs", Side::New, 82, 116, "Why this order?")
            .unwrap();

        let all = comments.all().unwrap().comments;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].body, "Why this order?");
    }

    #[test]
    fn an_empty_comment_is_refused() {
        let (_dir, comments) = comments();

        assert!(comments.add("src/a.rs", Side::New, 1, 1, "   \n ").is_err());
        assert!(comments.all().unwrap().comments.is_empty());
    }

    #[test]
    fn a_comment_on_a_file_outside_the_review_is_refused() {
        // The browser and the terminal both reach this, and neither should be
        // able to leave a note on a file the reviewer is not looking at.
        let (_dir, comments) = comments();

        let err = comments
            .add("elsewhere.rs", Side::New, 1, 1, "Why?")
            .unwrap_err();

        assert!(err.to_string().contains("elsewhere.rs"), "{err}");
        assert!(comments.all().unwrap().comments.is_empty());
    }

    #[test]
    fn a_comment_past_the_end_of_the_file_is_refused() {
        // It would render nowhere: there is no line to hang it under.
        let (_dir, comments) = comments();

        let err = comments
            .add("src/a.rs", Side::New, 300, 320, "Why?")
            .unwrap_err();

        assert!(err.to_string().contains("200"), "{err}");
        assert!(comments.all().unwrap().comments.is_empty());
    }

    #[test]
    fn a_comment_the_diff_has_stopped_reaching_is_not_listed() {
        // `add` proves the rule once and nothing holds it afterwards: the
        // reviewer pulls, the hunk the comment sat in is rewritten, and the
        // line it was anchored to is not printed any more.
        //
        // Left in the list it is the worst of both — counted in the margin
        // beside the file, drawn on no row because there is no row left, and so
        // impossible to read or to close.
        let dir = tempfile::tempdir().unwrap();
        let when_written = over_at(dir.path(), a_file());
        when_written
            .add("src/a.rs", Side::New, 12, 12, "Why this order?")
            .unwrap();
        assert_eq!(when_written.all().unwrap().comments.len(), 1);

        let after_pulling = over_at(dir.path(), a_file().showing("src/a.rs", vec![hunk(40, 3)]));

        assert!(after_pulling.all().unwrap().comments.is_empty());
    }

    #[test]
    fn a_comment_on_a_file_that_left_the_review_is_not_listed() {
        // The file has no diff to ask for at all. Asking would fail rather
        // than answer, and failing here would take the whole list down with it.
        let dir = tempfile::tempdir().unwrap();
        over_at(dir.path(), a_file())
            .add("src/a.rs", Side::New, 12, 12, "Why this order?")
            .unwrap();

        let elsewhere = over_at(dir.path(), FakeDiffSource::with_paths(&["src/b.rs"]));

        assert!(elsewhere.all().unwrap().comments.is_empty());
    }

    #[test]
    fn a_comment_on_a_line_the_diff_never_printed_is_refused() {
        // GitHub's rule, applied at writing time: a review comment can only sit
        // where the diff reaches. Caught here it costs one attempt; caught at
        // publishing it costs the comment.
        let (_dir, comments) = comments_showing(vec![hunk(10, 5), hunk(40, 3)]);

        let err = comments
            .add("src/a.rs", Side::New, 30, 30, "Why?")
            .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("30-30"), "{msg}");
        assert!(msg.contains("not in the diff"), "{msg}");
        assert!(comments.all().unwrap().comments.is_empty());
    }

    #[test]
    fn a_comment_spanning_the_gap_between_two_hunks_is_refused() {
        // Both ends land in the diff and the middle does not, which is the case
        // a check on the ends alone would wave through.
        let (_dir, comments) = comments_showing(vec![hunk(10, 5), hunk(40, 3)]);

        assert!(comments.add("src/a.rs", Side::New, 12, 41, "Why?").is_err());
        // And the span that stays inside one hunk goes in.
        assert!(comments.add("src/a.rs", Side::New, 12, 14, "Why?").is_ok());
    }

    #[test]
    fn a_comment_on_the_old_side_is_written_against_the_old_numbers() {
        // The diff prints old lines 40 to 42 and new lines 60 to 62. Asked on
        // the old side, 40 is in the diff and 60 is not — and the other way
        // round on the new one. Nothing here may quietly read one as the other.
        let (_dir, comments) = over(a_file().showing(
            "src/a.rs",
            vec![Hunk {
                old_start: 40,
                old_lines: 3,
                new_start: 60,
                new_lines: 3,
                lines: vec![],
            }],
        ));

        assert!(
            comments
                .add("src/a.rs", Side::Old, 40, 42, "Why drop this?")
                .is_ok()
        );
        assert!(comments.add("src/a.rs", Side::Old, 60, 60, "Why?").is_err());
        assert!(comments.add("src/a.rs", Side::New, 60, 62, "Why?").is_ok());
        assert!(comments.add("src/a.rs", Side::New, 40, 40, "Why?").is_err());

        let all = comments.all().unwrap().comments;
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].side, Side::Old);
        assert_eq!(all[1].side, Side::New);
    }

    #[test]
    fn a_file_that_was_deleted_whole_can_still_be_asked_about() {
        // Nothing is left on the new side, so the length of the file is zero
        // and a check against it would refuse every line there is. The diff is
        // what says where the old side reaches.
        let (_dir, comments) = over(
            FakeDiffSource::with_paths(&["gone.rs"])
                .with_line_count("gone.rs", 0)
                .showing(
                    "gone.rs",
                    vec![Hunk {
                        old_start: 1,
                        old_lines: 10,
                        new_start: 0,
                        new_lines: 0,
                        lines: vec![],
                    }],
                ),
        );

        assert!(
            comments
                .add("gone.rs", Side::Old, 3, 7, "Where did this go?")
                .is_ok()
        );
        assert!(
            comments.add("gone.rs", Side::Old, 3, 11, "Why?").is_err(),
            "past the end of what the diff printed"
        );
        assert!(
            comments.add("gone.rs", Side::New, 1, 1, "Why?").is_err(),
            "there is no new side to ask about"
        );
    }

    #[test]
    fn the_two_sides_keep_their_own_comments_at_the_same_numbers() {
        // The case the numbers alone cannot survive: one question about line 12
        // as it was and another about line 12 as it now reads.
        let (_dir, comments) = comments();
        comments
            .add("src/a.rs", Side::Old, 12, 12, "Why was this here?")
            .unwrap();
        comments
            .add("src/a.rs", Side::New, 12, 12, "Why is this here?")
            .unwrap();

        let all = comments.all().unwrap().comments;
        assert_eq!(all.len(), 2);
        assert_eq!(
            (all[0].side, all[1].side),
            (Side::Old, Side::New),
            "each kept the side it was written on"
        );
    }

    #[test]
    fn closing_a_comment_takes_it_off_the_list() {
        // Closing is answering, and an answered question is not a thing the
        // review keeps: the list is what is still waiting.
        let (_dir, comments) = comments();
        let one = comments.add("src/a.rs", Side::New, 1, 1, "Why?").unwrap();

        comments.close(&one.id).unwrap();

        assert!(comments.all().unwrap().comments.is_empty());
    }

    #[test]
    fn naming_a_comment_that_is_not_there_says_how_to_find_out() {
        let (_dir, comments) = comments();

        let err = comments.close("nope").unwrap_err().to_string();
        assert!(err.contains("nope"), "{err}");
        assert!(err.contains("comment list"), "{err}");
    }

    #[test]
    fn two_comments_written_together_do_not_share_an_id() {
        // They are written to travel: the reviewer's come back to the author
        // and the two sets are read side by side.
        let (_dir, comments) = comments();
        comments.add("src/a.rs", Side::New, 1, 1, "one").unwrap();
        comments.add("src/a.rs", Side::New, 2, 2, "two").unwrap();

        let all = comments.all().unwrap().comments;
        assert_ne!(all[0].id, all[1].id);
    }
}
