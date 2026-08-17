//! The wire, and what comes back on it.
//!
//! One agent for the life of the host, the headers every call carries, and a
//! reply kept whole — status beside body — because half of what this feature
//! reads is the host saying no.

use crate::comments::domain::CommentError;
use crate::error::Result;

/// Requests to one host. It holds the host so that a call that never arrives
/// can say which machine did not answer.
pub struct Http {
    host: String,
    agent: ureq::Agent,
}

impl Http {
    pub fn new(host: &str) -> Self {
        // A status is an answer, not a transport failure: a 404 means the
        // branch is not there, and that is something to read rather than to
        // fail on.
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .build();
        Self {
            host: host.to_string(),
            agent: ureq::Agent::new_with_config(config),
        }
    }

    pub fn get(&self, token: &str, url: &str) -> Result<Answer> {
        let answer = self
            .agent
            .get(url)
            .header("authorization", &format!("Bearer {token}"))
            .header("accept", ACCEPT)
            .header("x-github-api-version", VERSION)
            .header("user-agent", AGENT)
            .call();
        self.read(answer)
    }

    pub fn post(&self, token: &str, url: &str, body: String) -> Result<Answer> {
        let answer = self
            .agent
            .post(url)
            .header("authorization", &format!("Bearer {token}"))
            .header("accept", ACCEPT)
            .header("x-github-api-version", VERSION)
            .header("user-agent", AGENT)
            .header("content-type", "application/json")
            .send(body);
        self.read(answer)
    }

    fn read(
        &self,
        answer: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<Answer> {
        let mut answer = answer.map_err(|e| CommentError::Unreachable {
            host: self.host.clone(),
            why: e.to_string(),
        })?;
        let status = answer.status().as_u16();
        let body = answer.body_mut().read_to_string().unwrap_or_default();
        Ok(Answer { status, body })
    }
}

/// What came back, with the status kept beside the body so that a refusal can
/// be read rather than thrown.
pub struct Answer {
    status: u16,
    body: String,
}

impl Answer {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 401 is a token the host would not take; 403 is one it took and will not
    /// let do this. Both are answered by going and fixing the token, so they
    /// read the same from here.
    pub fn refused(&self) -> bool {
        self.status == 401 || self.status == 403
    }

    pub fn json(&self) -> Result<Option<serde_json::Value>> {
        Ok(serde_json::from_str(&self.body).ok())
    }

    /// What the host said was wrong, in its own words where it gave any. The
    /// `errors` array is where the useful half lives — the top-level message
    /// for a rejected comment is only ever "Validation Failed".
    pub fn complaint(&self) -> String {
        let Ok(Some(body)) = self.json() else {
            return format!("HTTP {}", self.status);
        };
        let message = body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("no reason given");
        let details: Vec<&str> = body
            .get("errors")
            .and_then(|e| e.as_array())
            .map(|errors| {
                errors
                    .iter()
                    .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        match details.is_empty() {
            true => message.to_string(),
            false => format!("{message} — {}", details.join("; ")),
        }
    }
}

/// Branch names carry slashes, and a slash in a query value is a path.
pub fn encoded(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// The version of the API this was written against. Pinned on purpose: without
/// it, a change GitHub announces for new clients arrives here unannounced.
const VERSION: &str = "2022-11-28";
const ACCEPT: &str = "application/vnd.github+json";
const AGENT: &str = concat!("farol/", env!("CARGO_PKG_VERSION"));
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejection_is_reported_in_the_hosts_own_words() {
        // "Validation Failed" alone would leave the reviewer nothing to act on;
        // the array underneath is where the reason lives.
        let answer = Answer {
            status: 422,
            body: serde_json::json!({
                "message": "Validation Failed",
                "errors": [{"message": "line must be part of the diff"}],
            })
            .to_string(),
        };

        let said = answer.complaint();
        assert!(said.contains("Validation Failed"), "{said}");
        assert!(said.contains("part of the diff"), "{said}");
        assert!(!answer.ok());
        assert!(!answer.refused());
    }

    #[test]
    fn a_body_that_is_not_json_still_says_the_status() {
        let answer = Answer {
            status: 502,
            body: "<html>bad gateway</html>".into(),
        };

        assert_eq!(answer.complaint(), "HTTP 502");
    }

    #[test]
    fn both_ways_of_saying_no_to_a_token_read_the_same() {
        // 401 is a token the host will not take, 403 is one it took and will
        // not let do this. Both are fixed by going and fixing the token.
        for status in [401, 403] {
            let answer = Answer {
                status,
                body: String::new(),
            };
            assert!(answer.refused(), "{status}");
        }
    }

    #[test]
    fn a_branch_name_with_a_slash_survives_being_put_in_a_url() {
        assert_eq!(encoded("feat/publish-review"), "feat%2Fpublish-review");
    }
}
