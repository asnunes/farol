mod message;
mod modes;
mod pull_request;
mod reach;
#[cfg(test)]
mod testing;
mod use_case;

pub use modes::*;
pub use pull_request::*;
pub use reach::*;
pub use use_case::*;

use std::sync::Arc;

use crate::comments::domain::{
    Author, Comment, CommentError, CommentStore, Found, Lines, PendingRef, Purpose, Subject,
};
use crate::diff::domain::Side;
use crate::error::Result;
use crate::lenses::domain::LensTarget;
use crate::map::application::GetScope;
use crate::map::domain::LineRange;

/// Everything that can be done to the reviewer's comments.
#[derive(Clone)]
pub struct Comments {
    store: Arc<dyn CommentStore>,
    /// Kept beside the reach rather than reached through it: what this needs it
    /// for is turning a raw path into a proven one, which is a different
    /// question from what the diff shows.
    scope: GetScope,
    reach: CommentReach,
}

impl Comments {
    pub fn new(store: Arc<dyn CommentStore>, scope: GetScope, reach: CommentReach) -> Self {
        Self {
            store,
            scope,
            reach,
        }
    }

    /// Every thread, split by whether the diff still reaches its lines, and
    /// the files that could not be read at all.
    ///
    /// `add` proves a thread sits on the diff, and nothing keeps that true: the
    /// branch moves, the lines go, and often it is the agent's own fix for the
    /// thread that moved them. Those threads used to be dropped here, which
    /// took the conversation off the screen and left the agent unable to
    /// answer. Now they come back on their own list, for the screen to draw
    /// away from the lines and for every command to still reach.
    ///
    /// Split on the read rather than in each caller, because this is the one
    /// the browser and the terminal both come through.
    pub fn all(&self) -> Result<Found> {
        let mut found = self.store.list()?;
        let split = self.reach.split(std::mem::take(&mut found.comments))?;
        found.comments = split.live;
        found.drifted = split.drifted;
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
        if !self.reach.shows(&path, side, from, to)? {
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
            about: Subject::Lines(Lines {
                path: path.as_str().to_string(),
                side,
                from,
                to,
            }),
            body: body.trim().to_string(),
            pending: None,
            conversation: Default::default(),
        };
        self.store.save(&comment)?;
        Ok(comment)
    }

    /// A thread on anything a reviewer can talk about. Lines go through
    /// `add`, which holds them to the diff; a lens or a block's description
    /// has no line to check, and a lens on a file is held to the review.
    pub fn add_about(&self, about: Subject, body: &str) -> Result<Comment> {
        if let Subject::Lines(lines) = &about {
            return self.add(&lines.path, lines.side, lines.from, lines.to, body);
        }
        if body.trim().is_empty() {
            return Err(CommentError::Empty.into());
        }
        if let Subject::Lens {
            target: LensTarget::File { path },
            ..
        } = &about
        {
            self.scope.path(path)?;
        }
        let comment = Comment {
            id: fresh_id(),
            about,
            body: body.trim().to_string(),
            pending: None,
            conversation: Default::default(),
        };
        self.store.save(&comment)?;
        Ok(comment)
    }

    /// One thread, wherever its lines are now, or the refusal that says how to
    /// find the ones there are.
    pub fn get(&self, id: &str) -> Result<Comment> {
        let found = self.all()?;
        found
            .comments
            .into_iter()
            .chain(found.drifted)
            .find(|comment| comment.id == id)
            .ok_or_else(|| CommentError::Unknown { id: id.into() }.into())
    }

    /// One thread whose lines the diff still prints, for what only makes sense
    /// on a line of the diff: a comment for the pull request.
    /// A thread on a lens or a block's description is refused the same way:
    /// it has no line to sit on.
    pub fn live(&self, id: &str) -> Result<Comment> {
        let found = self.all()?;
        if found.drifted.iter().any(|comment| comment.id == id) {
            return Err(CommentError::OffTheDiff { id: id.into() }.into());
        }
        let comment = found
            .comments
            .into_iter()
            .find(|comment| comment.id == id)
            .ok_or_else(|| CommentError::Unknown { id: id.into() })?;
        if comment.about.lines().is_none() {
            return Err(CommentError::NotOnCode {
                id: id.into(),
                about: comment.at(),
            }
            .into());
        }
        Ok(comment)
    }

    /// Every thread, on the diff or off it: what has a pending comment on the
    /// pull request does whether or not its lines are still shown.
    pub fn every(&self) -> Result<Vec<Comment>> {
        let found = self.all()?;
        Ok(found.comments.into_iter().chain(found.drifted).collect())
    }

    /// The thread's comment for the pull request is now this one.
    pub fn record_pending(&self, id: &str, pending: PendingRef) -> Result<Comment> {
        self.edit(id, |comment| comment.pending_written(pending))
    }

    /// The thread no longer has a comment for the pull request.
    pub fn clear_pending(&self, id: &str) -> Result<Comment> {
        self.edit(id, |comment| comment.pending = None)
    }

    /// Another turn in the conversation under a comment.
    pub fn reply(&self, id: &str, from: Author, body: &str) -> Result<Comment> {
        if body.trim().is_empty() {
            return Err(CommentError::Empty.into());
        }
        self.edit(id, |comment| comment.reply(from, body, &now()))
    }

    /// The agent says it has started on the comment.
    pub fn acknowledge(&self, id: &str, by: &str) -> Result<Comment> {
        self.edit(id, |comment| comment.acknowledged(by, &now()))
    }

    /// Record that the comment was handed to an agent, and what for.
    pub fn sent(&self, id: &str, to: &str, purpose: Purpose) -> Result<Comment> {
        self.edit(id, |comment| comment.sent(to, &now(), purpose))
    }

    /// One comment the review still holds, changed and written back. Read
    /// through `all` so a comment the diff no longer reaches cannot be answered
    /// either — it is not on the screen to be read.
    fn edit(&self, id: &str, change: impl FnOnce(&mut Comment)) -> Result<Comment> {
        let mut comment = self.get(id)?;
        change(&mut comment);
        self.store.save(&comment)?;
        Ok(comment)
    }

    /// Drop one. Whether it may go while a pending comment hangs off it is
    /// `CloseThread`'s question, which can ask the pull request.
    ///
    /// There is no settled-but-kept state. A comment lives as long as it is
    /// waiting for an answer, and the list is therefore always what is still
    /// open — nothing to filter, nothing to reopen.
    pub(super) fn close(&self, id: &str) -> Result<()> {
        match self.store.close(id)? {
            true => Ok(()),
            false => Err(CommentError::Unknown { id: id.into() }.into()),
        }
    }
}

/// Seconds since the epoch, the clock `viewed` is stamped with too.
fn now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
        .to_string()
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
        let scope = ReviewScope::new(source.clone());
        Comments::new(
            Arc::new(MarkdownComments::new(&store, dir)),
            GetScope::new(scope.clone()),
            CommentReach::new(scope, crate::diff::application::FileDiffs::new(source)),
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
    fn a_conversation_survives_the_store_and_an_agents_answer_settles_it() {
        let (_dir, comments) = comments();
        let one = comments
            .add("src/a.rs", Side::New, 82, 116, "Why this order?")
            .unwrap();

        comments
            .sent(&one.id, "Claude Code", Purpose::Answer)
            .unwrap();
        comments.acknowledge(&one.id, "Claude Code").unwrap();
        comments
            .reply(
                &one.id,
                Author::Agent("Claude Code".into()),
                "Because the **legacy** format.\n\nSecond paragraph.",
            )
            .unwrap();
        comments
            .reply(&one.id, Author::Reviewer, "And the new one?")
            .unwrap();

        let back = &comments.all().unwrap().comments[0];
        assert_eq!(
            back.body, "Why this order?",
            "replies must not leak into the question"
        );
        let replies = &back.conversation.replies;
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0].from, Author::Agent("Claude Code".into()));
        assert_eq!(
            replies[0].body,
            "Because the **legacy** format.\n\nSecond paragraph."
        );
        assert_eq!(replies[1].from, Author::Reviewer);
        assert_eq!(back.conversation.waiting, None);
    }

    #[test]
    fn a_reply_to_a_comment_that_is_not_there_says_how_to_find_them() {
        let (_dir, comments) = comments();
        let error = comments.reply("nope", Author::Reviewer, "hi").unwrap_err();
        assert!(error.to_string().contains("farol thread list"), "{error}");
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
    fn a_thread_the_diff_has_stopped_reaching_is_kept_apart_and_still_answered() {
        // `add` proves the rule once and nothing holds it afterwards: the
        // reviewer pulls, the hunk the thread sat in is rewritten — often by
        // the agent's own fix for it — and the line is not printed any more.
        // Dropped, the conversation would vanish and the answer would have
        // nowhere to go.
        let dir = tempfile::tempdir().unwrap();
        let when_written = over_at(dir.path(), a_file());
        let one = when_written
            .add("src/a.rs", Side::New, 12, 12, "Why this order?")
            .unwrap();

        let after_pulling = over_at(dir.path(), a_file().showing("src/a.rs", vec![hunk(40, 3)]));

        let found = after_pulling.all().unwrap();
        assert!(
            found.comments.is_empty(),
            "there is no line left to draw it at"
        );
        assert_eq!(found.drifted.len(), 1);
        after_pulling
            .reply(&one.id, Author::Agent("Codex".into()), "Moved it.")
            .unwrap();
        assert_eq!(
            after_pulling.all().unwrap().drifted[0]
                .conversation
                .replies
                .len(),
            1,
            "the agent's answer still lands on the thread"
        );
    }

    #[test]
    fn a_thread_on_a_file_that_left_the_review_is_kept_apart_without_asking_for_its_diff() {
        // The file has no diff to ask for at all. Asking would fail rather
        // than answer, and failing here would take the whole list down with it.
        let dir = tempfile::tempdir().unwrap();
        over_at(dir.path(), a_file())
            .add("src/a.rs", Side::New, 12, 12, "Why this order?")
            .unwrap();

        let elsewhere = over_at(dir.path(), FakeDiffSource::with_paths(&["src/b.rs"]));

        let found = elsewhere.all().unwrap();
        assert!(found.comments.is_empty());
        assert_eq!(found.drifted.len(), 1);
    }

    #[test]
    fn a_thread_off_the_diff_cannot_be_asked_for_as_a_live_one() {
        // What needs a line of the diff — a comment for the pull request —
        // is told why, rather than that the thread does not exist.
        let dir = tempfile::tempdir().unwrap();
        let one = over_at(dir.path(), a_file())
            .add("src/a.rs", Side::New, 12, 12, "Why this order?")
            .unwrap();

        let moved = over_at(dir.path(), a_file().showing("src/a.rs", vec![hunk(40, 3)]));

        let err = moved.live(&one.id).unwrap_err().to_string();
        assert!(err.contains("no longer shows"), "{err}");
        assert!(moved.get(&one.id).is_ok());
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
        assert_eq!(all[0].about.lines().unwrap().side, Side::Old);
        assert_eq!(all[1].about.lines().unwrap().side, Side::New);
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
            (
                all[0].about.lines().unwrap().side,
                all[1].about.lines().unwrap().side
            ),
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
        assert!(err.contains("thread list"), "{err}");
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

    #[test]
    fn a_thread_on_a_lens_is_kept_and_cannot_become_a_comment_for_the_pull_request() {
        use crate::lenses::domain::{LensId, LensTarget};
        let (_dir, comments) = comments();

        let one = comments
            .add_about(
                Subject::Lens {
                    lens: LensId::parse("overview").unwrap(),
                    target: LensTarget::Pr,
                },
                "This misses the cache.",
            )
            .unwrap();

        assert_eq!(
            comments.all().unwrap().comments.len(),
            1,
            "no line to drift from, so it stays"
        );
        let refused = comments.live(&one.id).unwrap_err().to_string();
        assert!(refused.contains("not lines of code"), "{refused}");
    }
}
