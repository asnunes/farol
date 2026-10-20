use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use crate::agent::domain::{AgentChannel, AgentError};

const HARNESS: &str = "Claude Code";

/// The inbox socket every Claude Code session opens for its own processes.
///
/// Two lines and a close: the session authenticates the token, queues the text
/// between tool calls or wakes up if idle, and writes nothing back. Nothing is
/// read, because there is nothing to read.
pub struct ClaudeSocket {
    socket: PathBuf,
    token: String,
}

impl ClaudeSocket {
    pub fn new(socket: PathBuf, token: String) -> Self {
        Self { socket, token }
    }

    fn lines(&self, text: &str) -> String {
        let auth = serde_json::json!({ "type": "auth", "token": self.token });
        let user = serde_json::json!({
            "type": "user",
            "message": { "role": "user", "content": text },
        });
        format!("{auth}\n{user}\n")
    }
}

impl AgentChannel for ClaudeSocket {
    fn harness(&self) -> &'static str {
        HARNESS
    }

    fn send(&self, text: &str) -> Result<(), AgentError> {
        let mut stream = UnixStream::connect(&self.socket).map_err(refusal)?;
        stream
            .write_all(self.lines(text).as_bytes())
            .map_err(refusal)?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(refusal)?;
        Ok(())
    }
}

/// A socket that is gone means the session is; anything else is the system
/// saying no, and it is passed on as it was said.
fn refusal(error: std::io::Error) -> AgentError {
    use std::io::ErrorKind::{ConnectionRefused, NotFound};
    match error.kind() {
        NotFound | ConnectionRefused => AgentError::Gone { harness: HARNESS },
        _ => AgentError::Refused {
            harness: HARNESS,
            why: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::unix::net::UnixListener;

    #[test]
    fn a_message_arrives_as_the_auth_line_then_the_user_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let received = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut all = String::new();
            stream.read_to_string(&mut all).unwrap();
            all
        });

        ClaudeSocket::new(path, "secret".into())
            .send("first line\nsecond line")
            .unwrap();

        let all = received.join().unwrap();
        let lines: Vec<serde_json::Value> = all
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            lines.len(),
            2,
            "a newline in the text must not split a frame: {all}"
        );
        assert_eq!(
            lines[0],
            serde_json::json!({"type": "auth", "token": "secret"})
        );
        assert_eq!(lines[1]["message"]["content"], "first line\nsecond line");
        assert_eq!(lines[1]["type"], "user");
    }

    #[test]
    fn a_socket_that_is_gone_says_to_serve_again() {
        let dir = tempfile::tempdir().unwrap();
        let error = ClaudeSocket::new(dir.path().join("gone.sock"), "secret".into())
            .send("hello")
            .unwrap_err();
        assert!(matches!(error, AgentError::Gone { .. }));
        assert!(error.to_string().contains("farol serve"), "{error}");
    }
}
