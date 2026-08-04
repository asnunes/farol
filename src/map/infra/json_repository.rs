use crate::map::domain::{MAP_VERSION, MapRepository, ReviewMap};
use crate::shared::error::Result;
use crate::shared::paths::Store;

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
        let map: Option<ReviewMap> = self.store.read_json(&path, |reason| {
            eprintln!("warning: discarding unreadable map at {sha}: {reason}");
        });

        // A map whose shape no longer matches is discarded rather than migrated:
        // the state dies with the worktree anyway, so migrations would be
        // ceremony. Saying so out loud still matters — silence would look like
        // the review had simply never been mapped.
        match map {
            Some(map) if map.version == MAP_VERSION => Ok(Some(map)),
            Some(map) => {
                eprintln!(
                    "warning: discarding map at {sha} — written by format version {}, this build speaks {MAP_VERSION}",
                    map.version
                );
                Ok(None)
            }
            None => Ok(None),
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
        self.store
            .write_json(&self.store.map_file(&map.generated_at), map)
    }

    fn delete(&self, sha: &str) -> Result<()> {
        let path = self.store.map_file(sha);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
