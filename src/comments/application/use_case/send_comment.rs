use std::sync::Arc;

use crate::agent::domain::{AgentChannel, AgentError};
use crate::comments::application::Comments;
use crate::comments::domain::{Author, Comment, CommentError};
use crate::config::domain::ConfigRepository;
use crate::error::Result;

/// Hand one comment to the session that implemented the branch.
///
/// The browser sends an id and nothing else; the text is built here from the
/// comment as stored, so the route cannot be used to put arbitrary words in
/// front of an agent that may be running without permission prompts.
#[derive(Clone)]
pub struct SendComment {
    comments: Comments,
    channel: Option<Arc<dyn AgentChannel>>,
    config: Arc<dyn ConfigRepository>,
}

impl SendComment {
    pub fn new(
        comments: Comments,
        channel: Option<Arc<dyn AgentChannel>>,
        config: Arc<dyn ConfigRepository>,
    ) -> Self {
        Self {
            comments,
            channel,
            config,
        }
    }

    /// Which harness a press would reach, for the button to say so — or none,
    /// and then there is no button.
    pub fn harness(&self) -> Option<&'static str> {
        self.channel.as_ref().map(|channel| channel.harness())
    }

    /// Read the instruction on every send rather than once: a change made from
    /// the terminal has to count on the next press, without restarting the
    /// server. An unreadable configuration stops the send instead of falling
    /// back to the default, which would put words the user replaced in front of
    /// the agent.
    pub fn execute(&self, id: &str) -> Result<()> {
        let channel = self.channel.as_ref().ok_or(AgentError::NoSession)?;
        let comment = self
            .comments
            .all()?
            .comments
            .into_iter()
            .find(|comment| comment.id == id)
            .ok_or_else(|| CommentError::Unknown { id: id.into() })?;
        let config = self.config.read()?;
        channel.send(&message(&comment, config.send_instruction()))?;
        // Only once the transport took it: a comment marked as sent that never
        // left would sit on screen waiting for an answer nobody was asked for.
        self.comments.sent(&comment.id, channel.harness())?;
        Ok(())
    }
}

/// The part farol always writes, and then whatever the user chose to add.
///
/// Fixed: naming farol, so the skill is picked up; the id, so the instruction
/// never has to carry it; the conversation so far, so a follow-up stands on its
/// own; and the two commands that bring the answer back to the screen. Those
/// last are not the user's to edit — an instruction that dropped them would
/// leave the comment marked as waiting forever, with the answer only in the
/// agent's terminal.
fn message(comment: &Comment, instruction: &str) -> String {
    let id = &comment.id;
    let mut text = format!(
        "A reviewer left a comment on this branch in farol (comment {id}):\n\n{}",
        comment.quoted()
    );
    if !comment.conversation.replies.is_empty() {
        text.push_str("\n\nThe conversation so far, oldest first:");
        for reply in &comment.conversation.replies {
            let who = match &reply.from {
                Author::Reviewer => "Reviewer",
                Author::Agent(name) => name,
            };
            text.push_str(&format!("\n\n{who}: {}", reply.body));
        }
    }
    text.push_str(&format!(
        "\n\nBefore you start, run `farol comment ack {id}` so the reviewer sees you are on it. Answer with `farol comment reply {id} --text '…'`, in single quotes: that is where the reviewer reads it."
    ));
    match instruction.trim() {
        "" => text,
        instruction => format!("{text}\n\n{instruction}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::application::CommentReach;
    use crate::comments::infra::MarkdownComments;
    use crate::config::domain::{SettingKey, SettingValue};
    use crate::diff::application::{FileDiffs, ReviewScope};
    use crate::diff::domain::Side;
    use crate::map::application::GetScope;
    use crate::shared::paths::Store;
    use crate::testing::{
        BrokenConfigRepository, FakeDiffSource, InMemoryConfigRepository, RecordingChannel,
    };

    struct World {
        _dir: tempfile::TempDir,
        comments: Comments,
        channel: Arc<RecordingChannel>,
        config: Arc<InMemoryConfigRepository>,
    }

    impl World {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::new(dir.path(), "feature/x");
            let source = Arc::new(
                FakeDiffSource::with_paths(&["src/a.rs"]).with_line_count("src/a.rs", 200),
            );
            let scope = ReviewScope::new(source.clone());
            let comments = Comments::new(
                Arc::new(MarkdownComments::new(&store, dir.path())),
                GetScope::new(scope.clone()),
                CommentReach::new(scope, FileDiffs::new(source)),
            );
            Self {
                _dir: dir,
                comments,
                channel: Arc::new(RecordingChannel::default()),
                config: Arc::new(InMemoryConfigRepository::default()),
            }
        }

        fn send(&self) -> SendComment {
            SendComment::new(
                self.comments.clone(),
                Some(self.channel.clone()),
                self.config.clone(),
            )
        }

        fn instruct(&self, text: &str) {
            use crate::config::domain::ConfigRepository as _;
            let mut config = self.config.read().unwrap();
            config
                .set(
                    SettingKey::parse("send-instruction").unwrap(),
                    SettingValue::Text(text.into()),
                )
                .unwrap();
            self.config.write(&config).unwrap();
        }

        fn comment(&self) -> Comment {
            self.comments
                .add("src/a.rs", Side::New, 82, 116, "Why this order?")
                .unwrap()
        }
    }

    #[test]
    fn the_message_carries_where_the_comment_is_its_id_and_the_default_instruction() {
        let world = World::new();
        let comment = world.comment();

        world.send().execute(&comment.id).unwrap();

        assert_eq!(
            world.channel.sent(),
            vec![format!(
                "A reviewer left a comment on this branch in farol (comment {id}):\n\nsrc/a.rs:82-116\n\nWhy this order?\n\nBefore you start, run `farol comment ack {id}` so the reviewer sees you are on it. Answer with `farol comment reply {id} --text '…'`, in single quotes: that is where the reviewer reads it.\n\nAnswer it, or change the code and say what you changed.",
                id = comment.id
            )]
        );
    }

    #[test]
    fn the_saved_instruction_follows_the_comment_and_a_change_counts_on_the_next_send() {
        let world = World::new();
        let comment = world.comment();

        world.instruct("Only answer. Leave it open.");
        world.send().execute(&comment.id).unwrap();
        world.instruct("Fix it.");
        world.send().execute(&comment.id).unwrap();

        let sent = world.channel.sent();
        assert!(
            sent[0].ends_with("reads it.\n\nOnly answer. Leave it open."),
            "{}",
            sent[0]
        );
        assert!(sent[1].ends_with("reads it.\n\nFix it."), "{}", sent[1]);
    }

    #[test]
    fn an_empty_instruction_sends_only_the_fixed_part() {
        let world = World::new();
        let comment = world.comment();
        world.instruct("   ");

        world.send().execute(&comment.id).unwrap();

        assert!(world.channel.sent()[0].ends_with("that is where the reviewer reads it."));
    }

    #[test]
    fn a_send_marks_the_comment_as_waiting_and_a_follow_up_carries_the_conversation() {
        use crate::comments::domain::Stage;
        let world = World::new();
        let comment = world.comment();

        world.send().execute(&comment.id).unwrap();
        let waiting = world.comments.all().unwrap().comments[0]
            .conversation
            .waiting
            .clone();
        assert_eq!(
            waiting.map(|w| (w.on, w.stage)),
            Some(("Claude Code".into(), Stage::Sent))
        );

        world
            .comments
            .reply(
                &comment.id,
                Author::Agent("Claude Code".into()),
                "Because of X.",
            )
            .unwrap();
        world
            .comments
            .reply(&comment.id, Author::Reviewer, "And Y?")
            .unwrap();
        world.send().execute(&comment.id).unwrap();

        let second = &world.channel.sent()[1];
        assert!(
            second.contains("Claude Code: Because of X.\n\nReviewer: And Y?"),
            "a follow-up has to stand on its own: {second}"
        );
    }

    #[test]
    fn a_failed_send_leaves_the_comment_not_waiting() {
        let world = World::new();
        let comment = world.comment();
        let send = SendComment::new(
            world.comments.clone(),
            Some(Arc::new(crate::testing::RefusingChannel)),
            world.config.clone(),
        );

        assert!(send.execute(&comment.id).is_err());
        assert_eq!(
            world.comments.all().unwrap().comments[0]
                .conversation
                .waiting,
            None
        );
    }

    #[test]
    fn an_unreadable_configuration_refuses_to_send() {
        let world = World::new();
        let comment = world.comment();
        let send = SendComment::new(
            world.comments.clone(),
            Some(world.channel.clone()),
            Arc::new(BrokenConfigRepository),
        );

        assert!(send.execute(&comment.id).is_err());
        assert!(
            world.channel.sent().is_empty(),
            "the default must not go out in place of an instruction the user wrote"
        );
    }

    #[test]
    fn an_unknown_comment_sends_nothing_and_says_how_to_list_them() {
        let world = World::new();

        let error = world.send().execute("nope").unwrap_err();

        assert!(error.to_string().contains("farol comment list"), "{error}");
        assert!(world.channel.sent().is_empty());
    }

    #[test]
    fn without_a_session_the_refusal_points_back_to_copy() {
        let world = World::new();
        let comment = world.comment();
        let send = SendComment::new(world.comments.clone(), None, world.config.clone());

        assert_eq!(send.harness(), None);
        let error = send.execute(&comment.id).unwrap_err();
        assert!(error.to_string().contains("copy the comment"), "{error}");
    }
}
