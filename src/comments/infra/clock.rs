use crate::comments::domain::Clock;

/// The wall clock, and the process it runs in.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> String {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default()
            .to_string()
    }

    /// The clock plus the process, which two machines cannot both produce.
    /// Comments are meant to travel: the reviewer's come back to the author,
    /// and two sets get read together, so a counter would collide the moment
    /// that happens.
    fn id(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        format!("{now:x}-{:x}", std::process::id())
    }
}
