use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the session that ran `farol serve` can be reached.
///
/// Read off the environment the harness exports to its shell, never typed:
/// the token would otherwise pass through the model, and the model could hand
/// over another session's. It travels in the session configuration so that a
/// review already open follows the session that last asked for it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "harness")]
pub enum AgentAddress {
    ClaudeCode { socket: PathBuf, token: String },
    Codex { thread: String },
}

impl AgentAddress {
    /// Claude Code only with both halves: Codex strips variables named like a
    /// token from its shell, so a Codex session opened inside Claude Code
    /// inherits the socket without the token, and should be the one answering.
    pub fn from_env(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let set = |name: &str| var(name).filter(|value| !value.is_empty());
        if let (Some(socket), Some(token)) = (
            set("CLAUDE_CODE_MESSAGING_SOCKET"),
            set("CLAUDE_CODE_MESSAGING_TOKEN"),
        ) {
            return Some(Self::ClaudeCode {
                socket: socket.into(),
                token,
            });
        }
        set("CODEX_THREAD_ID").map(|thread| Self::Codex { thread })
    }
}

/// The token is a credential for the session, so it never reaches a log.
impl std::fmt::Debug for AgentAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClaudeCode { socket, .. } => f
                .debug_struct("ClaudeCode")
                .field("socket", socket)
                .field("token", &"<redacted>")
                .finish(),
            Self::Codex { thread } => f.debug_struct("Codex").field("thread", thread).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn the_claude_socket_wins_when_both_harnesses_left_their_variables() {
        let address = AgentAddress::from_env(env(&[
            ("CLAUDE_CODE_MESSAGING_SOCKET", "/tmp/cc.sock"),
            ("CLAUDE_CODE_MESSAGING_TOKEN", "secret"),
            ("CODEX_THREAD_ID", "thread-1"),
        ]));
        assert!(matches!(address, Some(AgentAddress::ClaudeCode { .. })));
    }

    #[test]
    fn a_socket_without_its_token_is_not_a_claude_session() {
        let address = AgentAddress::from_env(env(&[
            ("CLAUDE_CODE_MESSAGING_SOCKET", "/tmp/cc.sock"),
            ("CODEX_THREAD_ID", "thread-1"),
        ]));
        assert_eq!(
            address,
            Some(AgentAddress::Codex {
                thread: "thread-1".into()
            }),
            "Codex strips the token from its shell, so this is Codex inside Claude"
        );
    }

    #[test]
    fn an_empty_variable_counts_as_absent() {
        assert_eq!(
            AgentAddress::from_env(env(&[
                ("CLAUDE_CODE_MESSAGING_SOCKET", ""),
                ("CLAUDE_CODE_MESSAGING_TOKEN", ""),
                ("CODEX_THREAD_ID", ""),
            ])),
            None
        );
    }

    #[test]
    fn without_either_harness_there_is_no_one_to_send_to() {
        assert_eq!(AgentAddress::from_env(env(&[])), None);
    }

    #[test]
    fn the_token_never_shows_in_debug_output() {
        let address = AgentAddress::ClaudeCode {
            socket: "/tmp/cc.sock".into(),
            token: "secret".into(),
        };
        assert!(!format!("{address:?}").contains("secret"));
    }
}
