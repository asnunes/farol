use crate::progress::domain::{PROGRESS_VERSION, Progress, ProgressRepository};
use crate::shared::error::Result;
use crate::shared::paths::{Store, write_atomic};

pub struct JsonProgressRepository {
    store: Store,
}

impl JsonProgressRepository {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl ProgressRepository for JsonProgressRepository {
    fn load(&self) -> Result<Progress> {
        let Ok(raw) = std::fs::read_to_string(self.store.state_file()) else {
            return Ok(Progress::new());
        };
        match serde_json::from_str::<Progress>(&raw) {
            Ok(p) if p.version == PROGRESS_VERSION => Ok(p),
            // Losing read-state is an inconvenience, not a disaster: worst case
            // the reviewer re-ticks what they already read.
            _ => Ok(Progress::new()),
        }
    }

    fn save(&self, progress: &Progress) -> Result<()> {
        self.store.ensure()?;
        let body = serde_json::to_vec_pretty(progress)?;
        write_atomic(&self.store.state_file(), &body)
    }
}
