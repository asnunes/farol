use crate::error::{Error, Result};

use super::{ServerEntry, SessionConfig};

impl ServerEntry {
    pub fn configure(&self, config: &SessionConfig) -> Result<Self> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build()
            .into();
        let body = serde_json::to_vec(config)?;
        let mut response = agent
            .put(format!("{}/api/session", self.url()))
            .header("content-type", "application/json")
            .send(&body)
            .map_err(|e| {
                Error::msg(format!(
                    "cannot update {}: {e}\nCheck the running server with `farol servers`.",
                    self.url()
                ))
            })?;
        let status = response.status();
        let body = response.body_mut().read_to_string().map_err(|e| {
            Error::msg(format!("cannot read the response from {}: {e}", self.url()))
        })?;
        if !status.is_success() {
            return Err(Error::msg(format!("cannot update {}: {body}", self.url())));
        }
        Ok(serde_json::from_str(&body)?)
    }
}
