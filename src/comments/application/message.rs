//! What farol writes to the agent, for each thing it can be asked.

use crate::comments::domain::{Author, Comment, ReviewMode};

/// Asking for an answer in the thread.
///
/// Fixed: naming farol, so the skill is picked up; the id, so the instruction
/// never has to carry it; the conversation so far, so a follow-up stands on its
/// own; and the two commands that bring the answer back to the screen. Those
/// are not the user's to edit — an instruction that dropped them would leave
/// the thread waiting forever, with the answer only in the agent's terminal.
pub fn answer(comment: &Comment, mode: ReviewMode, instruction: &str) -> String {
    let id = &comment.id;
    let text = format!(
        "A reviewer left a comment on this branch in farol (thread {id}):\n\n{}{}{}\n\nBefore you start, run `farol thread ack {id}` so the reviewer sees you are on it. Answer with `farol thread reply {id} --text '…'`, in single quotes: that is where the reviewer reads it.",
        comment.quoted(),
        shown(comment),
        conversation(comment),
    );
    finish(text, &[instruction], mode)
}

/// Asking for the thread's comment for the pull request.
///
/// The comment it replaces travels with it, when there is one: a regenerate
/// is usually "the same, but", and the agent cannot see what it is replacing
/// any other way.
pub fn pending(
    comment: &Comment,
    replacing: Option<&str>,
    mode: ReviewMode,
    instruction: &str,
    this_time: &str,
) -> String {
    let id = &comment.id;
    let replacing = match replacing {
        Some(body) => format!("\n\nIt replaces this pending comment:\n\n{}", body.trim()),
        None => String::new(),
    };
    let text = format!(
        "A reviewer wants a comment for the pull request out of a thread in farol (thread {id}):\n\n{}{}{replacing}\n\nBefore you start, run `farol thread ack {id}` so the reviewer sees you are on it. Write the comment with `farol pending write {id} --text '…'`, in single quotes. It goes into the reviewer's pending review on the pull request, which only they see until they submit the review. Do not submit the review or post anything else on the pull request.",
        comment.quoted(),
        conversation(comment),
    );
    let this_time = match this_time.trim() {
        "" => String::new(),
        extra => format!("For this one: {extra}"),
    };
    finish(text, &[instruction, &this_time], mode)
}

/// For a thread on a lens or a block's description: how to read what it is
/// about, and that rewriting it is part of answering when the reviewer is
/// right.
fn shown(comment: &Comment) -> String {
    match comment.about.shown_by() {
        Some(command) => format!(
            "\n\nThe thread is about text you wrote, not code: read it with `{command}`, and when the reviewer is right, rewrite it as well as answering."
        ),
        None => String::new(),
    }
}

/// Asking for the pull request's title and description.
///
/// The command that brings them back is fixed, like the others: without it the
/// draft would only ever reach the agent's terminal. Opening the pull request
/// is the reviewer's to do, from the page, so the agent is told not to.
pub fn pull_draft(branch: &str, base: &str, mode: ReviewMode, instruction: &str) -> String {
    let text = format!(
        "A reviewer wants to open a pull request for {branch} into {base} from farol, and asks you to write its title and description. Read the change with `git diff {base}...{branch}`, and the map with `farol map show` when there is one.\n\nWrite them with `farol github draft --title '…' --text '…'`, in single quotes: the reviewer reads and edits them in farol before opening the pull request. Do not open the pull request, push or post anything yourself."
    );
    finish(text, &[instruction], mode)
}

/// The conversation under the question, oldest first.
fn conversation(comment: &Comment) -> String {
    if comment.conversation.replies.is_empty() {
        return String::new();
    }
    let mut text = String::from("\n\nThe conversation so far, oldest first:");
    for reply in &comment.conversation.replies {
        let who = match &reply.from {
            Author::Reviewer => "Reviewer",
            Author::Agent(name) => name,
        };
        text.push_str(&format!("\n\n{who}: {}", reply.body));
    }
    text
}

/// The user's words, then the mode's rule. The rule goes last because it wins:
/// an instruction in Settings that says "fix it" must not reach an agent in a
/// peer review as the final word.
fn finish(text: String, instructions: &[&str], mode: ReviewMode) -> String {
    let mut text = text;
    for instruction in instructions
        .iter()
        .map(|i| i.trim())
        .filter(|i| !i.is_empty())
    {
        text.push_str("\n\n");
        text.push_str(instruction);
    }
    text.push_str("\n\n");
    text.push_str(mode.rule());
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::{Lines, Subject};
    use crate::diff::domain::Side;

    fn comment() -> Comment {
        Comment {
            id: "t1".into(),
            about: Subject::Lines(Lines {
                path: "src/a.rs".into(),
                side: Side::New,
                from: 3,
                to: 4,
            }),
            body: "Why?".into(),
            pending: None,
            conversation: Default::default(),
        }
    }

    #[test]
    fn the_peer_rule_is_the_last_word_whatever_settings_say() {
        let text = answer(&comment(), ReviewMode::PeerReview, "Fix it and push.");
        assert!(
            text.ends_with("do not change, commit or push code in this repository."),
            "{text}"
        );
        let empty = pending(&comment(), None, ReviewMode::PeerReview, "", "");
        assert!(empty.ends_with(ReviewMode::PeerReview.rule()), "{empty}");
    }

    #[test]
    fn a_regenerate_carries_what_it_replaces_and_the_instruction_for_this_time() {
        let text = pending(
            &comment(),
            Some("Rename it."),
            ReviewMode::PeerReview,
            "Be kind.",
            "shorter",
        );
        assert!(
            text.contains("It replaces this pending comment:\n\nRename it."),
            "{text}"
        );
        assert!(text.contains("farol pending write t1 --text"), "{text}");
        assert!(text.contains("Be kind.\n\nFor this one: shorter"), "{text}");
    }
}
