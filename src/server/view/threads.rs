//! The threads, under the file each one is about, and the files that could not
//! be read as one.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::comments::domain::{Author, Comment, Found, Purpose, Stage};
use crate::comments::presentation::says;
use crate::diff::domain::Side;

/// The comments, under the file each one is about, and what the store could not
/// read at all.
///
/// Keyed by path rather than handed over as one flat list, for the reason every
/// other answer here is shaped before it goes: the screen asks one question of
/// this — what is on *this* file — and asking it of a flat list means grouping
/// in the browser, in as many places as ask. The count beside a file and the
/// boxes under its lines then come from the same array without anyone having to
/// keep two derivations agreeing.
///
/// Within a file they are oldest first. Ids carry the clock, so ordering by id
/// is ordering by when the question was asked, and that is the order a reader
/// works through them in.
///
/// An object rather than a bare map because the unreadable files travel with
/// it: the page has to be able to say a comment went missing, and a map keyed
/// by reviewed path has nowhere to put one whose path is exactly what broke.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentsView {
    pub files: BTreeMap<String, Vec<CommentView>>,
    /// Threads whose lines the diff no longer prints, oldest first. Not under
    /// their file, because there is no line left in it to draw them at: the
    /// screen puts them at the end of the file's block, or in a block of their
    /// own when the file has none.
    pub drifted: Vec<CommentView>,
    pub unreadable: Vec<UnreadableView>,
    /// The harness a Send would reach, or null — and then there is no button.
    /// Carried here because the only reader is the comment's own row.
    pub agent: Option<&'static str>,
}

impl CommentsView {
    pub fn of(found: &Found, agent: Option<&'static str>) -> Self {
        let mut files: BTreeMap<String, Vec<CommentView>> = BTreeMap::new();
        for comment in &found.comments {
            files
                .entry(comment.path.clone())
                .or_default()
                .push(CommentView::of(comment));
        }
        for here in files.values_mut() {
            here.sort_by(|a, b| a.id.cmp(&b.id));
        }

        Self {
            files,
            drifted: found.drifted.iter().map(CommentView::of).collect(),
            unreadable: found
                .unreadable
                .iter()
                .map(|one| UnreadableView {
                    file: one.file.clone(),
                    about: one.about.clone(),
                    excerpt: one.excerpt.clone(),
                    why: says(one.why),
                })
                .collect(),
            agent,
        }
    }
}

/// One comment, for the wire.
///
/// The domain keeps its own shape: comments are stored as markdown, so serde on
/// `Comment` would exist for nothing but this hop.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentView {
    pub id: String,
    pub path: String,
    /// Which side of the diff `from` and `to` are counted on. The screen draws
    /// the comment in the column it was written in, and the two columns number
    /// separately.
    pub side: Side,
    pub from: u32,
    pub to: u32,
    pub body: String,
    /// Who wrote the thread's comment for the pull request, when it has one.
    /// What it says and whether it went are on the pull request, and arrive
    /// with `PullRequestView`.
    pub pending: Option<PendingByView>,
    /// The turns after the question, oldest first.
    pub replies: Vec<ReplyView>,
    /// Set while an agent owes an answer, for the mark on the comment.
    pub waiting: Option<WaitingView>,
}

impl CommentView {
    pub fn of(comment: &Comment) -> Self {
        Self {
            id: comment.id.clone(),
            path: comment.path.clone(),
            side: comment.side,
            from: comment.from,
            to: comment.to,
            body: comment.body.clone(),
            pending: comment.pending.as_ref().map(|pending| match &pending.by {
                Author::Reviewer => PendingByView {
                    id: pending.id.clone(),
                    by: "reviewer",
                    name: "you".into(),
                },
                Author::Agent(name) => PendingByView {
                    id: pending.id.clone(),
                    by: "agent",
                    name: name.clone(),
                },
            }),
            replies: comment
                .conversation
                .replies
                .iter()
                .map(|reply| match &reply.from {
                    Author::Reviewer => ReplyView {
                        from: "reviewer",
                        name: "You".into(),
                        body: reply.body.clone(),
                        at: reply.at.clone(),
                    },
                    Author::Agent(name) => ReplyView {
                        from: "agent",
                        name: name.clone(),
                        body: reply.body.clone(),
                        at: reply.at.clone(),
                    },
                })
                .collect(),
            waiting: comment
                .conversation
                .waiting
                .as_ref()
                .map(|waiting| WaitingView {
                    on: waiting.on.clone(),
                    stage: match waiting.stage {
                        Stage::Sent => "sent",
                        Stage::Working => "working",
                    },
                    purpose: match waiting.purpose {
                        Purpose::Answer => "answer",
                        Purpose::Pending => "pending",
                    },
                    since: waiting.since.clone(),
                }),
        }
    }
}

/// One turn of a comment's conversation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyView {
    /// "reviewer" or "agent", which decides which side of the exchange it reads on.
    pub from: &'static str,
    /// Who to name it as: "You" for the reviewer, the harness for an agent.
    pub name: String,
    pub body: String,
    /// Seconds since the epoch, for the activity list to say how long ago.
    pub at: String,
}

/// Something owed, as the screen marks it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingView {
    pub on: String,
    /// "sent" until the agent acknowledges, "working" after.
    pub stage: &'static str,
    /// "answer" or "pending": what the badge says is being done.
    pub purpose: &'static str,
    /// Seconds since the epoch.
    pub since: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingByView {
    /// GitHub's id for the comment, which tells one pending comment from the
    /// one that replaced it.
    pub id: String,
    /// "reviewer" or "agent".
    pub by: &'static str,
    /// "you", or the harness.
    pub name: String,
}

/// A comment the store could not read, in the terms the page speaks: the file
/// it was written about, or the start of the prose when the header no longer
/// says which file that was.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadableView {
    pub file: String,
    pub about: Option<String>,
    pub excerpt: Option<String>,
    pub why: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_go_out_under_the_file_they_are_about() {
        // Shaped here so the browser looks a file up. Handed over flat, every
        // place that wants one file's comments groups the list again, and the
        // count beside a file stops being the array drawn under its lines.
        let found = Found {
            comments: vec![
                comment("18cb-0002", "src/a.rs"),
                comment("18cb-0003", "src/b.rs"),
                comment("18cb-0001", "src/a.rs"),
            ],
            drifted: Vec::new(),
            unreadable: Vec::new(),
        };

        let view = CommentsView::of(&found, None);

        assert_eq!(view.files.len(), 2);
        assert_eq!(
            ids(&view, "src/a.rs"),
            vec!["18cb-0001", "18cb-0002"],
            "oldest first: the id carries the clock, and the reader works \
             through the questions in the order they were asked"
        );
        assert_eq!(ids(&view, "src/b.rs"), vec!["18cb-0003"]);
    }

    #[test]
    fn a_file_nobody_commented_on_is_simply_absent() {
        // The page asks for a key that is not there and gets nothing, which is
        // what "no questions on this file" has to look like.
        let view = CommentsView::of(&Found::default(), None);

        assert!(view.files.is_empty());
    }

    fn comment(id: &str, path: &str) -> Comment {
        Comment {
            id: id.into(),
            path: path.into(),
            side: Side::New,
            from: 1,
            to: 1,
            body: "Por que essa ordem?".into(),
            pending: None,
            conversation: Default::default(),
        }
    }

    fn ids<'a>(view: &'a CommentsView, path: &str) -> Vec<&'a str> {
        view.files[path].iter().map(|c| c.id.as_str()).collect()
    }
}
