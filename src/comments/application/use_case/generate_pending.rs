use std::sync::Arc;

use crate::agent::domain::{AgentChannel, AgentError};
use crate::comments::application::{Comments, ReviewModes, message};
use crate::comments::domain::{Purpose, RemoteState, ReviewPublisher};
use crate::config::domain::ConfigRepository;
use crate::error::Result;

/// Ask the agent to write a thread's comment for the pull request.
///
/// The agent writes it back with `farol pending write`, which goes through
/// `WritePending` like a comment the reviewer typed — so what reaches GitHub is
/// held to the same rules whoever wrote it.
#[derive(Clone)]
pub struct GeneratePending {
    comments: Comments,
    channel: Option<Arc<dyn AgentChannel>>,
    config: Arc<dyn ConfigRepository>,
    modes: ReviewModes,
    publisher: Arc<dyn ReviewPublisher>,
}

impl GeneratePending {
    pub fn new(
        comments: Comments,
        channel: Option<Arc<dyn AgentChannel>>,
        config: Arc<dyn ConfigRepository>,
        modes: ReviewModes,
        publisher: Arc<dyn ReviewPublisher>,
    ) -> Self {
        Self {
            comments,
            channel,
            config,
            modes,
            publisher,
        }
    }

    /// `this_time` is the reviewer's note for this one comment, added after the
    /// instruction from Settings rather than in place of it.
    pub fn execute(&self, id: &str, this_time: &str) -> Result<()> {
        let channel = self.channel.as_ref().ok_or(AgentError::NoSession)?;
        let thread = self.comments.get(id)?;
        let config = self.config.read()?;
        let mode = self.modes.current()?.mode;
        // What it replaces, read off the pull request when farol can. Without
        // it the agent still writes a comment; it just cannot write "the same,
        // but shorter".
        let replacing = thread
            .pending
            .as_ref()
            .and_then(|pending| {
                self.publisher
                    .remote(std::slice::from_ref(&pending.id))
                    .ok()
            })
            .and_then(|remote| match remote.into_iter().next()?.state {
                RemoteState::Pending { body } => Some(body),
                _ => None,
            });
        channel.send(&message::pending(
            &thread,
            replacing.as_deref(),
            mode,
            config.pending_instruction(),
            this_time,
        ))?;
        self.comments
            .sent(&thread.id, channel.harness(), Purpose::Pending)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::comments::application::testing::World;
    use crate::comments::domain::{Author, Purpose};

    #[test]
    fn the_agent_is_asked_for_a_pending_comment_and_the_thread_waits_for_one() {
        let world = World::ready();
        let thread = world.thread();

        world.generate_pending().execute(&thread.id, "").unwrap();

        let sent = &world.channel.sent()[0];
        assert!(
            sent.contains(&format!("farol pending write {}", thread.id)),
            "{sent}"
        );
        assert!(
            sent.contains("kindly and briefly"),
            "the instruction from Settings: {sent}"
        );
        let waiting = world.comments.get(&thread.id).unwrap().conversation.waiting;
        assert_eq!(waiting.map(|w| w.purpose), Some(Purpose::Pending));
    }

    #[test]
    fn a_regenerate_shows_the_agent_what_it_replaces() {
        let world = World::ready();
        let thread = world.thread();
        world
            .write_pending()
            .execute(
                &thread.id,
                "Rename it.",
                Author::Agent("Claude Code".into()),
            )
            .unwrap();

        world
            .generate_pending()
            .execute(&thread.id, "shorter")
            .unwrap();

        let sent = &world.channel.sent()[0];
        assert!(
            sent.contains("It replaces this pending comment:\n\nRename it."),
            "{sent}"
        );
        assert!(sent.contains("For this one: shorter"), "{sent}");
    }
}
