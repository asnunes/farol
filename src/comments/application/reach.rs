//! Whether the diff still prints the lines a comment sits on.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::comments::domain::Comment;
use crate::diff::application::{FileDiffs, ReviewScope};
use crate::diff::domain::{FileDiff, ReviewPath, Side};
use crate::error::Result;

/// The one place that asks whether the diff reaches a comment.
///
/// The question comes up at three moments — writing one, listing what is open,
/// and sending the review — and each of them used to work it out on its own.
/// That is how the screen came to count a comment in the margin that no row
/// could draw: two of the three agreed and the third was never asked.
///
/// It holds the scope and the diffs for the reason `MapReconciler` does: only
/// this side can see the code, and the answer is no use to anything that
/// cannot.
#[derive(Clone)]
pub struct CommentReach {
    scope: ReviewScope,
    diffs: FileDiffs,
}

impl CommentReach {
    pub fn new(scope: ReviewScope, diffs: FileDiffs) -> Self {
        Self { scope, diffs }
    }

    /// Whether every line of a span appears in the diff, counted on the side it
    /// was written on.
    ///
    /// Takes a path already proven to be under review, because the caller that
    /// asks this is writing a comment and has turned it into one to do so. An
    /// unproven path has no diff to ask for at all, which is the other method's
    /// problem and not this one's.
    pub fn shows(&self, path: &ReviewPath, side: Side, from: u32, to: u32) -> Result<bool> {
        Ok(self.diffs.of(path.as_str())?.shows(side, from, to))
    }

    /// Split comments by whether the diff still reaches them.
    ///
    /// Splits rather than filters because the two callers want opposite halves:
    /// listing keeps what is still printed, and publishing takes what drifted so
    /// it can name each one. Handing back only one would send the other to
    /// compute the same thing again, which is how the rule came to be written
    /// twice in the first place.
    ///
    /// A path that left the review window is decided before the diff is asked
    /// for: `file_diff` refuses one rather than reporting that it shows
    /// nothing, and that refusal would take the whole list down with it — the
    /// reviewer would lose sight of every other comment over one that drifted.
    ///
    /// One diff per file, not one per comment: a file with a dozen questions on
    /// it would otherwise be diffed a dozen times.
    pub fn split(&self, comments: Vec<Comment>) -> Result<Reached> {
        let scope = self.scope.get()?;
        let mut seen: HashMap<String, Option<FileDiff>> = HashMap::new();
        let mut split = Reached::default();

        for comment in comments {
            let diff = match seen.entry(comment.path.clone()) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(match scope.contains(&comment.path) {
                    true => Some(self.diffs.of(&comment.path)?),
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
}

/// Comments the diff still prints, and comments it has stopped printing.
#[derive(Debug, Default)]
pub struct Reached {
    pub live: Vec<Comment>,
    pub drifted: Vec<Comment>,
}
