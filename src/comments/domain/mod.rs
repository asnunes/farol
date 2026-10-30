mod error;
mod mode;
mod publishing;
mod pull_request;
mod subject;

pub use error::*;
pub use mode::*;
pub use publishing::*;
pub use pull_request::*;
pub use subject::*;

use crate::error::Result;

/// Something the reviewer wrote about the change — a thread, on screen. About
/// a span of lines, a lens or a block's description: see [`Subject`].
///
/// Theirs, not the session's: the map carries what the author explained, and
/// this carries what the reader asked back. They sit in different files for the
/// same reason they read differently on screen.
///
/// Still called a comment here, though the screen says thread: the name is
/// what the store, the routes and the files on disk are all spelled in, and
/// renaming every one of them would buy nothing the screen's word does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub id: String,
    pub about: Subject,
    /// Markdown, kept as written.
    pub body: String,
    /// The comment for the pull request that came out of this thread, which
    /// lives in the reviewer's pending review on GitHub rather than here.
    ///
    /// Only its id is kept. The text and whether it went are GitHub's to say,
    /// so there is no copy to fall out of step with the page of the pull
    /// request, where the reviewer can edit it too.
    pub pending: Option<PendingRef>,
    /// What was said after the question, and whether an agent owes an answer.
    pub conversation: Conversation,
}

impl Comment {
    /// Handed to an agent; it owes something from here on — an answer, or a
    /// pending comment, which `purpose` says.
    pub fn sent(&mut self, to: &str, at: &str, purpose: Purpose) {
        self.conversation.waiting = Some(Waiting {
            on: to.to_string(),
            since: at.to_string(),
            stage: Stage::Sent,
            purpose,
        });
    }

    /// The agent says it has started. Kept even when farol never recorded a
    /// send — the agent may have been handed the comment some other way, and
    /// the reviewer is better told it is being worked on than not.
    pub fn acknowledged(&mut self, by: &str, at: &str) {
        let (on, purpose) = match &self.conversation.waiting {
            Some(waiting) => (waiting.on.clone(), waiting.purpose),
            None => (by.to_string(), Purpose::Answer),
        };
        self.conversation.waiting = Some(Waiting {
            on,
            since: at.to_string(),
            stage: Stage::Working,
            purpose,
        });
    }

    /// Another turn. An agent's turn settles an answer it owed; the reviewer's
    /// does not, because writing back is not the same as sending. Nor does an
    /// agent's reply settle a pending comment it was asked for: talking about
    /// the comment is not writing it, and the badge would go away with nothing
    /// on the pull request.
    pub fn reply(&mut self, from: Author, body: &str, at: &str) {
        if matches!(from, Author::Agent(_)) && self.owes(Purpose::Answer) {
            self.conversation.waiting = None;
        }
        self.conversation.replies.push(Reply {
            from,
            body: body.trim().to_string(),
            at: at.to_string(),
        });
    }

    /// A pending comment now stands for this thread. It settles a pending
    /// comment the agent owed, and nothing else: an answer still owed is still
    /// owed.
    pub fn pending_written(&mut self, pending: PendingRef) {
        if self.owes(Purpose::Pending) {
            self.conversation.waiting = None;
        }
        self.pending = Some(pending);
    }

    fn owes(&self, purpose: Purpose) -> bool {
        self.conversation
            .waiting
            .as_ref()
            .is_some_and(|waiting| waiting.purpose == purpose)
    }

    /// What lands on the clipboard: enough for the answer to be read somewhere
    /// else without the file open.
    pub fn quoted(&self) -> String {
        format!("{}\n\n{}", self.at(), self.body.trim())
    }

    /// Where it sits, the way a person would say it.
    pub fn at(&self) -> String {
        self.about.at()
    }

    /// The same thread on the other column, for tests that need one.
    #[cfg(test)]
    pub fn on_side(mut self, side: crate::diff::domain::Side) -> Self {
        if let Subject::Lines(lines) = &mut self.about {
            lines.side = side;
        }
        self
    }
}

/// The exchange under a comment, oldest first, and what is pending.
///
/// A conversation rather than one answer: an agent's reply often settles one
/// thing and raises another, and sending a second comment about the same lines
/// would split one question across two places.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conversation {
    pub replies: Vec<Reply>,
    /// Set while an agent owes an answer; gone once it gives one.
    pub waiting: Option<Waiting>,
}

/// One turn after the question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub from: Author,
    /// Markdown, as written.
    pub body: String,
    /// Seconds since the epoch, as text: the same clock `viewed` is stamped with.
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Author {
    Reviewer,
    /// The agent, by the name of its harness: "Claude Code", "Codex".
    Agent(String),
}

/// Something owed, and how far along it is as far as farol was told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    /// The harness it was sent to.
    pub on: String,
    pub since: String,
    pub stage: Stage,
    pub purpose: Purpose,
}

/// What the agent was asked for, which decides what settles the wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// An answer in the thread: `farol thread reply`.
    Answer,
    /// A comment for the pull request: `farol pending write`.
    Pending,
}

/// Where a thread's comment for the pull request lives, and who wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRef {
    /// GitHub's node id for the review comment.
    pub id: String,
    pub by: Author,
}

/// Sent is what farol knows on its own; working is what the agent says when it
/// acknowledges. Nothing in between can be known: neither harness reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Sent,
    Working,
}

/// Where comments are kept. A trait for the same reason the map has one: the
/// use cases should not know that this is a folder of files.
pub trait CommentStore: Send + Sync {
    fn list(&self) -> Result<Found>;
    fn save(&self, comment: &Comment) -> Result<()>;
    /// Whether there was one to close.
    fn close(&self, id: &str) -> Result<bool>;
}

/// What a read of the store turned up.
///
/// The comments are only half of it. These are files a person is invited to
/// open and edit, so one of them will eventually come back malformed — and one
/// bad file must not cost the whole review. It gets skipped, and it gets named:
/// skipping quietly is how a comment somebody wrote disappears without a word,
/// which is worse than the error it was avoiding.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Found {
    /// Threads on lines the diff still prints.
    pub comments: Vec<Comment>,
    /// Threads whose lines the diff no longer prints: the code moved, or the
    /// file left the review. Kept apart rather than dropped, because the
    /// conversation is still the reader's and the agent may still be answering.
    pub drifted: Vec<Comment>,
    pub unreadable: Vec<Unreadable>,
}

/// A file that could not be turned into a comment, and everything that can
/// still be said about it.
///
/// What broke is the header, which is the part that says where the comment
/// belongs — so half the time there is no reviewed file left to name. When
/// there is, it is what the reviewer is told, because that is the language the
/// review is read in. When there is not, they get the start of what they wrote,
/// which is how a person recognises their own comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// The file to open to fix it, from where the reviewer is standing.
    pub file: String,
    /// The reviewed file it was written about, when the header still says.
    pub about: Option<String>,
    /// The start of what they wrote, when it does not.
    pub excerpt: Option<String>,
    pub why: Unread,
}

/// What the header is missing. A closed set, so the wording lives with the
/// presentation rather than being built where the file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unread {
    /// No `---` header at all.
    NoHeader,
    /// Nothing saying which file it is about.
    NoPath,
    /// Nothing saying which lines, or something that is not a range.
    NoLines,
    /// A side that is neither `old` nor `new`. A missing one is not this: it
    /// means the new side, which is all farol could write for a long time.
    /// Something else written there is a guess nobody should make, because
    /// guessing wrong puts the comment on the column the reviewer was not
    /// reading.
    BadSide,
    /// A lens or a block that cannot be read back: an id farol would not
    /// write, or a lens with nothing saying what it is on.
    BadSubject,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::domain::Side;

    fn comment(from: u32, to: u32) -> Comment {
        Comment {
            id: "1".into(),
            about: Subject::Lines(Lines {
                path: "src/a.rs".into(),
                side: Side::New,
                from,
                to,
            }),
            body: "  Why this order?  ".into(),
            pending: None,
            conversation: Conversation::default(),
        }
    }

    #[test]
    fn a_copied_comment_carries_where_it_was_about() {
        // Pasted into a chat or an issue, it has to stand on its own.
        assert_eq!(
            comment(82, 116).quoted(),
            "src/a.rs:82-116\n\nWhy this order?"
        );
    }

    #[test]
    fn a_comment_on_one_line_says_one_line() {
        assert_eq!(comment(82, 82).quoted(), "src/a.rs:82\n\nWhy this order?");
    }

    #[test]
    fn one_on_the_old_side_says_so_where_the_numbers_are() {
        // Line 82 exists on both sides of the same file. Pasted somewhere the
        // diff is not open, the numbers alone would send the reader to the
        // wrong one.
        let removed = crate::comments::domain::Comment::on_side(comment(82, 116), Side::Old);

        assert_eq!(removed.at(), "src/a.rs:82-116 (old)");
        assert_eq!(
            crate::comments::domain::Comment::on_side(comment(9, 9), Side::Old).at(),
            "src/a.rs:9 (old)"
        );
    }

    #[test]
    fn an_agents_reply_settles_what_it_owed_and_the_reviewers_does_not() {
        let mut one = comment(1, 1);
        one.sent("Claude Code", "10", Purpose::Answer);
        one.acknowledged("Claude Code", "11");
        assert_eq!(
            one.conversation.waiting.as_ref().unwrap().stage,
            Stage::Working
        );

        one.reply(Author::Reviewer, "And this?", "12");
        assert!(
            one.conversation.waiting.is_some(),
            "writing back is not sending: the agent still owes the first answer"
        );

        one.reply(Author::Agent("Claude Code".into()), "Because.", "13");
        assert_eq!(one.conversation.waiting, None);
        assert_eq!(one.conversation.replies.len(), 2);
    }

    #[test]
    fn each_wait_is_settled_only_by_what_it_was_waiting_for() {
        let mut one = comment(1, 1);
        one.sent("Codex", "10", Purpose::Pending);
        one.acknowledged("Codex", "11");
        one.reply(Author::Agent("Codex".into()), "Writing it now.", "12");
        assert!(
            one.conversation.waiting.is_some(),
            "a reply is not the pending comment the agent was asked for"
        );

        one.pending_written(PendingRef {
            id: "PRRC_1".into(),
            by: Author::Agent("Codex".into()),
        });
        assert_eq!(one.conversation.waiting, None);

        one.sent("Codex", "13", Purpose::Answer);
        one.pending_written(PendingRef {
            id: "PRRC_2".into(),
            by: Author::Reviewer,
        });
        assert!(
            one.conversation.waiting.is_some(),
            "the reviewer writing a pending comment does not answer for the agent"
        );
        assert_eq!(one.pending.unwrap().id, "PRRC_2");
    }
}
