//! How a thread is written as markdown, and read back.
//!
//! Apart from the store, which only knows where the files are: this is the
//! format, and the store is the folder. A change to how a header line is
//! spelled is a change here and nowhere else.

use crate::comments::domain::{
    Author, Comment, Conversation, PendingRef, Purpose, Reply, Stage, Unread, Unreadable, Waiting,
};
use crate::diff::domain::Side;

/// The file a thread is kept as.
pub(super) fn render(comment: &Comment) -> String {
    let pending = match &comment.pending {
        Some(PendingRef {
            id,
            by: Author::Reviewer,
        }) => format!("pending: {id} reviewer\n"),
        Some(PendingRef {
            id,
            by: Author::Agent(name),
        }) => format!("pending: {id} agent {name}\n"),
        None => String::new(),
    };
    // The purpose on a line of its own, and only when it is not an answer:
    // every file written before there was a second purpose reads as it did.
    let waiting = match &comment.conversation.waiting {
        Some(w) => format!(
            "waiting: {} {} {}\n{}",
            stage(w.stage),
            w.since,
            w.on,
            match w.purpose {
                Purpose::Answer => "",
                Purpose::Pending => "waiting-for: pending\n",
            }
        ),
        None => String::new(),
    };
    let header = format!("{pending}{waiting}");
    // Each turn under a marker a markdown viewer does not draw, so the file
    // still reads as prose in an editor and the turns stay apart.
    let replies: String = comment
        .conversation
        .replies
        .iter()
        .map(|reply| {
            let who = match &reply.from {
                Author::Reviewer => format!("reviewer {}", reply.at),
                Author::Agent(name) => format!("agent {} {name}", reply.at),
            };
            format!("\n{REPLY}{who} -->\n\n{}\n", reply.body.trim())
        })
        .collect();
    // The side is written every time, including the one that is the default.
    // This is a file somebody opens and edits, and a key that appears only
    // sometimes is a key they have to know about before they can use it.
    format!(
        "---\npath: {}\nside: {}\nlines: {}-{}\n{header}---\n\n{}\n",
        comment.path,
        comment.side,
        comment.from,
        comment.to,
        comment.body.trim()
    ) + &replies
}

/// Where a turn of the conversation starts in the file.
const REPLY: &str = "<!-- farol:reply ";

fn stage(stage: Stage) -> &'static str {
    match stage {
        Stage::Sent => "sent",
        Stage::Working => "working",
    }
}

/// `sent 1734567 Claude Code`: the stage, when, and the harness, which may
/// have spaces in its name and so goes last.
fn waiting(raw: &str) -> Option<Waiting> {
    let mut parts = raw.splitn(3, ' ');
    let stage = match parts.next()? {
        "sent" => Stage::Sent,
        "working" => Stage::Working,
        _ => return None,
    };
    let since = parts.next()?.to_string();
    let on = parts.next().unwrap_or("agent").trim().to_string();
    Some(Waiting {
        on,
        since,
        stage,
        purpose: Purpose::Answer,
    })
}

/// `PRRC_kw 1 reviewer` or `PRRC_kw agent Claude Code`: the id GitHub gave the
/// comment, then who wrote it.
fn pending(raw: &str) -> Option<PendingRef> {
    let mut parts = raw.splitn(3, ' ');
    let id = parts.next()?.to_string();
    let by = match parts.next()? {
        "reviewer" => Author::Reviewer,
        "agent" => Author::Agent(parts.next().unwrap_or("agent").trim().to_string()),
        _ => return None,
    };
    Some(PendingRef { id, by })
}

/// The question, then each turn under its marker. A marker that cannot be read
/// is kept as part of the turn before it rather than dropping what follows.
fn conversation(body: &str) -> (String, Vec<Reply>) {
    let mut question = String::new();
    let mut replies: Vec<Reply> = Vec::new();
    for line in body.lines() {
        let turn = line
            .strip_prefix(REPLY)
            .and_then(|rest| rest.strip_suffix("-->"))
            .and_then(|who| {
                let mut parts = who.trim().splitn(3, ' ');
                let kind = parts.next()?;
                let at = parts.next()?.to_string();
                let from = match kind {
                    "reviewer" => Author::Reviewer,
                    "agent" => Author::Agent(parts.next().unwrap_or("agent").trim().to_string()),
                    _ => return None,
                };
                Some(Reply {
                    from,
                    body: String::new(),
                    at,
                })
            });
        match (turn, replies.last_mut()) {
            (Some(turn), _) => replies.push(turn),
            (None, Some(last)) => {
                last.body.push_str(line);
                last.body.push('\n');
            }
            (None, None) => {
                question.push_str(line);
                question.push('\n');
            }
        }
    }
    for reply in &mut replies {
        reply.body = reply.body.trim().to_string();
    }
    (question.trim().to_string(), replies)
}

/// The header, then the prose. Written by hand rather than with a yaml crate:
/// a handful of keys does not justify a dependency, and a file a person edited
/// by hand and got slightly wrong should be skipped, not fatal.
///
/// Anything in the header that is not one of the keys it knows is passed over: a
/// key from an older farol, a line somebody was drafting, a blank one. Only the
/// facts it needs can stop it — and when they do, what survived comes back so
/// the reviewer is told in their own terms rather than in file names.
pub(super) fn parse(id: &str, file: String, raw: &str) -> std::result::Result<Comment, Unreadable> {
    let unreadable = |why, about: Option<String>, body: &str| Unreadable {
        file: file.clone(),
        about,
        excerpt: excerpt(body),
        why,
    };

    let Some((head, body)) = raw
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
    else {
        // Nothing to read but the prose, which is all the reviewer has left to
        // recognise it by.
        return Err(unreadable(Unread::NoHeader, None, raw));
    };

    let mut path = None;
    let mut side = None;
    let mut lines = None;
    let mut pending_ref = None;
    let mut owed = None;
    let mut purpose = Purpose::Answer;
    // `published:` from 2.0.0-rc.1 falls through with the other unknown keys:
    // that comment went out with a review nothing here can edit any more.
    for (key, value) in head.lines().filter_map(|line| line.split_once(':')) {
        match key.trim() {
            "path" => path = Some(value.trim().to_string()),
            "side" => side = Some(value.trim().to_string()),
            "lines" => lines = Some(value.trim().to_string()),
            "pending" => pending_ref = pending(value.trim()),
            "waiting" => owed = waiting(value.trim()),
            "waiting-for" if value.trim() == "pending" => purpose = Purpose::Pending,
            _ => {}
        }
    }
    let owed = owed.map(|waiting| Waiting { purpose, ..waiting });

    let Some(path) = path else {
        return Err(unreadable(Unread::NoPath, None, body));
    };
    // No side is the new one: every comment farol wrote before it could write
    // any other was about the file as it now reads, so reading those files that
    // way is reading them correctly rather than defaulting. A side that is
    // there and says something else is a different matter — it is skipped and
    // named, because putting the comment on the wrong column is worse than
    // telling the reviewer one file needs a look.
    let side = match side.as_deref() {
        None => Side::New,
        Some(raw) => match Side::parse(raw) {
            Some(side) => side,
            None => return Err(unreadable(Unread::BadSide, Some(path), body)),
        },
    };
    let range = lines
        .as_deref()
        .map(|l| l.split_once('-').unwrap_or((l, l)))
        .and_then(|(a, b)| Some((a.trim().parse().ok()?, b.trim().parse().ok()?)));
    let Some((from, to)) = range else {
        return Err(unreadable(Unread::NoLines, Some(path), body));
    };

    let (question, replies) = conversation(body);
    Ok(Comment {
        id: id.to_string(),
        path,
        side,
        from,
        to,
        body: question,
        pending: pending_ref,
        conversation: Conversation {
            replies,
            waiting: owed,
        },
    })
}

/// How much of a comment it takes to recognise it. One line, because that is
/// what a chip and a terminal row have room for, and the first line of a
/// comment is where people put the question.
const EXCERPT: usize = 60;

fn excerpt(body: &str) -> Option<String> {
    let line = body.trim().lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    match line.chars().count() > EXCERPT {
        true => Some(line.chars().take(EXCERPT).collect::<String>() + "…"),
        false => Some(line.to_string()),
    }
}
