//! Asking a registered server whether it is really there.
//!
//! The registry says what was started; this says what is serving. They part
//! company more often than they should: a server that was asked to stop can
//! stop listening without ending, a machine can go to sleep with reviews open,
//! and a port that was let go can be taken by something else entirely.
//!
//! Spoken over a socket by hand rather than through an HTTP client, because one
//! request to a port on this machine does not justify a dependency the rest of
//! the program has no use for.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use super::registry::ServerEntry;

/// Long enough for a loaded machine, short enough that a list of dead entries
/// still comes back at once.
const TO_ANSWER: Duration = Duration::from_millis(500);

/// Whether this entry describes a server that is answering, and is the one the
/// entry claims it is.
pub fn answers(entry: &ServerEntry) -> bool {
    match identity(entry.port) {
        Some(found) => found.pid == entry.pid,
        None => false,
    }
}

fn identity(port: u16) -> Option<ServerEntry> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut socket = TcpStream::connect_timeout(&address, TO_ANSWER).ok()?;
    socket.set_read_timeout(Some(TO_ANSWER)).ok()?;
    socket.set_write_timeout(Some(TO_ANSWER)).ok()?;

    // HTTP/1.0: the server closes when it has finished, so the read ends on its
    // own and nothing here has to understand keep-alive or chunked bodies.
    socket
        .write_all(format!("GET /health HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\n\r\n").as_bytes())
        .ok()?;

    let mut answer = Vec::new();
    socket.read_to_end(&mut answer).ok()?;
    let answer = String::from_utf8(answer).ok()?;

    let (head, body) = answer.split_once("\r\n\r\n")?;
    match head.lines().next()?.contains(" 200 ") {
        true => serde_json::from_str(body).ok(),
        false => None,
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::io::BufRead;
    use std::net::TcpListener;
    use std::path::PathBuf;

    fn entry(port: u16, pid: u32) -> ServerEntry {
        ServerEntry {
            port,
            pid,
            repo: PathBuf::from("/repo"),
            branch: "feature/x".into(),
            base: "main".into(),
        }
    }

    /// Something on a port that answers `/health` the way farol does, for as
    /// long as the test needs it. Shared with the registry's tests, which
    /// cannot ask about a running server without one.
    pub(in crate::server) fn server_answering(with: Option<ServerEntry>) -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            while let Ok((mut socket, _)) = listener.accept() {
                let mut request = String::new();
                std::io::BufReader::new(socket.try_clone().unwrap())
                    .read_line(&mut request)
                    .unwrap();

                let response = match &with {
                    Some(entry) => {
                        let body = serde_json::to_string(entry).unwrap();
                        format!("HTTP/1.0 200 OK\r\ncontent-type: application/json\r\n\r\n{body}")
                    }
                    None => "HTTP/1.0 404 Not Found\r\n\r\n".to_string(),
                };
                let _ = socket.write_all(response.as_bytes());
            }
        });

        port
    }

    #[test]
    fn a_server_that_answers_as_itself_is_there() {
        let port = server_answering(Some(entry(4600, 42)));

        assert!(answers(&entry(port, 42)));
    }

    #[test]
    fn a_port_with_nothing_on_it_is_not() {
        // The commonest case by far: the entry outlived the server.
        let free = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = free.local_addr().unwrap().port();
        drop(free);

        assert!(!answers(&entry(port, 42)));
    }

    #[test]
    fn somebody_else_on_the_same_port_is_not_the_server_that_was_registered() {
        // Ports get reused. Without the process id this would report the
        // neighbour's review as if it were yours.
        let port = server_answering(Some(entry(4600, 999)));

        assert!(!answers(&entry(port, 42)));
    }

    #[test]
    fn a_reply_that_is_not_an_answer_settles_nothing() {
        let port = server_answering(None);

        assert!(!answers(&entry(port, 42)));
    }
}
