//! What reaching a session refuses, in its own vocabulary.

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error(
        "no agent session is connected to this review\nAsk the agent in Claude Code or Codex to run `farol serve`, or copy the comment instead."
    )]
    NoSession,

    #[error(
        "the {harness} session that started this review is no longer listening\nAsk the agent in the session that should receive comments to run `farol serve`; the open review will follow it."
    )]
    Gone { harness: &'static str },

    #[error("cannot run codex\nInstall Codex CLI, or copy the comment instead.")]
    Unavailable,

    #[error("{harness} would not take the message: {why}")]
    Refused { harness: &'static str, why: String },
}
