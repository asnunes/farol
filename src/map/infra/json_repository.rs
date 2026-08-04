use crate::map::domain::{MAP_VERSION, MapRepository, ReviewMap};
use crate::shared::error::Result;
use crate::shared::paths::{Store, write_atomic};

pub struct JsonMapRepository {
    store: Store,
}

impl JsonMapRepository {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl MapRepository for JsonMapRepository {
    fn load_at(&self, sha: &str) -> Result<Option<ReviewMap>> {
        let path = self.store.map_file(sha);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Ok(None);
        };

        // A map whose shape no longer matches is discarded rather than migrated:
        // the state dies with the worktree anyway, so migrations would be
        // ceremony. Saying so out loud still matters — silence would look like
        // the review had simply never been mapped.
        match serde_json::from_str::<ReviewMap>(&raw) {
            Ok(map) if map.version == MAP_VERSION => Ok(Some(map)),
            Ok(map) => {
                eprintln!(
                    "warning: discarding map at {sha} — written by format version {}, this build speaks {MAP_VERSION}",
                    map.version
                );
                Ok(None)
            }
            Err(e) => {
                eprintln!("warning: discarding unreadable map at {sha}: {e}");
                Ok(None)
            }
        }
    }

    fn stored_shas(&self) -> Result<Vec<String>> {
        let dir = self.store.maps_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(sha) = name.strip_suffix(".json") {
                out.push(sha.to_string());
            }
        }
        out.sort();
        Ok(out)
    }

    fn save(&self, map: &ReviewMap) -> Result<()> {
        self.store.ensure()?;
        let body = serde_json::to_vec_pretty(map)?;
        write_atomic(&self.store.map_file(&map.generated_at), &body)
    }

    fn delete(&self, sha: &str) -> Result<()> {
        let path = self.store.map_file(sha);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
