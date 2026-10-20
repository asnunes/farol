use std::sync::Arc;

use crate::agent::domain::{AgentChannel, AgentError};
use crate::comments::application::Comments;
use crate::comments::domain::{Comment, CommentError};
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
        Ok(())
    }
}

/// The part farol always writes — naming farol, so the skill is picked up, and
/// carrying the id, so the instruction never has to — and then whatever the
/// user chose to add.
fn message(comment: &Comment, instruction: &str) -> String {
    let head = format!(
        "A reviewer left a comment on this branch in farol (comment {}):\n\n{}",
        comment.id,
        comment.quoted()
    );
    match instruction.trim() {
        "" => head,
        instruction => format!("{head}\n\n{instruction}"),
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
                "A reviewer left a comment on this branch in farol (comment {}):\n\nsrc/a.rs:82-116\n\nWhy this order?\n\nAnswer it, or change the code. Once it is resolved, close it with `farol comment close` and its id.",
                comment.id
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
            sent[0].ends_with("Why this order?\n\nOnly answer. Leave it open."),
            "{}",
            sent[0]
        );
        assert!(
            sent[1].ends_with("Why this order?\n\nFix it."),
            "{}",
            sent[1]
        );
    }

    #[test]
    fn an_empty_instruction_sends_only_the_fixed_part() {
        let world = World::new();
        let comment = world.comment();
        world.instruct("   ");

        world.send().execute(&comment.id).unwrap();

        assert!(world.channel.sent()[0].ends_with("Why this order?"));
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
