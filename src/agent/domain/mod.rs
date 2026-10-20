mod error;

pub use error::*;

/// The session on the other end, whichever harness it runs in.
///
/// One implementation per harness, chosen in the composition root from what the
/// harness exported into the environment. Whoever sends does not learn which.
pub trait AgentChannel: Send + Sync {
    /// How the screen names it: "Send to Claude Code".
    fn harness(&self) -> &'static str;

    /// Hand the text to the harness as if the user had typed it.
    ///
    /// `Ok` means the transport took it, and no more: Codex confirms that the
    /// message is queued, Claude Code's socket confirms nothing, and the common
    /// contract keeps the weaker of the two. Nothing here says the agent read it.
    fn send(&self, text: &str) -> Result<(), AgentError>;
}
