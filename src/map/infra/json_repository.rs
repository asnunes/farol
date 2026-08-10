use crate::error::Result;
use crate::map::domain::{MAP_VERSION, MapRepository, ReviewMap};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::{Position, WORKING};
    use crate::testing::slug;

    fn repo() -> (tempfile::TempDir, JsonMapRepository) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path(), "feature/x");
        (dir, JsonMapRepository::new(store))
    }

    fn mapped(sha: &str) -> ReviewMap {
        let mut m = ReviewMap::new("feature/x", "main", sha);
        m.add_block(&slug("core"), "The change", "why", Position::End)
            .unwrap();
        m.add_file(&slug("core"), "a.rs", Some("a note".into()), None)
            .unwrap();
        m
    }

    #[test]
    fn a_map_survives_being_written_and_read_back_whole() {
        let (_dir, repo) = repo();

        repo.save(&mapped("abc123")).unwrap();

        let back = repo.load_at("abc123").unwrap().expect("it was just saved");
        assert_eq!(back.generated_at, "abc123");
        assert_eq!(back.branch, "feature/x");
        let block = back.block(&slug("core")).unwrap();
        assert_eq!(block.title, "The change");
        assert_eq!(block.context, "why", "the prose is the expensive part");
        assert_eq!(block.file("a.rs").unwrap().note.as_deref(), Some("a note"));
    }

    #[test]
    fn a_version_that_was_never_written_is_simply_absent() {
        // Ordinary: it is how deriving learns there is nothing here yet.
        let (_dir, repo) = repo();

        assert!(repo.load_at("abc123").unwrap().is_none());
    }

    #[test]
    fn a_map_is_filed_under_the_commit_it_was_built_against() {
        let (_dir, repo) = repo();

        repo.save(&mapped("abc123")).unwrap();

        assert!(repo.load_at("abc123").unwrap().is_some());
        assert!(repo.load_at("def456").unwrap().is_none());
    }

    #[test]
    fn a_map_written_by_another_format_version_is_discarded_not_misread() {
        // The shape may have moved under it. Reading it anyway would put a
        // half-understood map in front of the reviewer, which is worse than
        // saying there is none.
        let (dir, repo) = repo();
        let store = Store::new(dir.path(), "feature/x");
        let mut future = mapped("abc123");
        future.version = MAP_VERSION + 1;
        store
            .write_json(&store.map_file("abc123"), &future)
            .unwrap();

        assert!(repo.load_at("abc123").unwrap().is_none());
    }

    #[test]
    fn a_file_cut_short_is_discarded_rather_than_taking_the_process_down() {
        // A crash mid-write, or a full disk.
        let (dir, repo) = repo();
        let store = Store::new(dir.path(), "feature/x");
        store.ensure().unwrap();
        std::fs::write(store.map_file("abc123"), "{ \"version\": 1, \"blocks\": [").unwrap();

        assert!(repo.load_at("abc123").unwrap().is_none());
    }

    #[test]
    fn the_stored_versions_come_back_sorted_and_named_by_sha() {
        let (_dir, repo) = repo();
        for sha in ["ccc", "aaa", "bbb"] {
            repo.save(&mapped(sha)).unwrap();
        }

        assert_eq!(repo.stored_shas().unwrap(), vec!["aaa", "bbb", "ccc"]);
    }

    #[test]
    fn uncommitted_work_is_stored_alongside_the_commits() {
        // It is keyed by a sentinel rather than a sha, and has to be listed
        // like any other version or derivation would not find it to inherit.
        let (_dir, repo) = repo();
        repo.save(&mapped(WORKING)).unwrap();

        assert_eq!(repo.stored_shas().unwrap(), vec![WORKING]);
        assert!(repo.load_at(WORKING).unwrap().is_some());
    }

    #[test]
    fn a_store_that_was_never_written_to_lists_nothing() {
        // The directory does not exist yet on a fresh branch; that is not a
        // failure to report.
        let (_dir, repo) = repo();

        assert!(repo.stored_shas().unwrap().is_empty());
    }

    #[test]
    fn files_that_are_not_maps_are_ignored() {
        let (dir, repo) = repo();
        let store = Store::new(dir.path(), "feature/x");
        repo.save(&mapped("abc123")).unwrap();
        std::fs::write(store.maps_dir().join("notes.txt"), "scratch").unwrap();

        assert_eq!(repo.stored_shas().unwrap(), vec!["abc123"]);
    }

    #[test]
    fn deleting_a_version_takes_it_out_of_the_listing_too() {
        let (_dir, repo) = repo();
        repo.save(&mapped("abc123")).unwrap();

        repo.delete("abc123").unwrap();

        assert!(repo.load_at("abc123").unwrap().is_none());
        assert!(repo.stored_shas().unwrap().is_empty());
    }

    #[test]
    fn deleting_something_that_is_not_there_is_not_an_error() {
        // Deriving absorbs the working map by deleting it, and it may not be
        // there; refusing would fail a command that did nothing wrong.
        let (_dir, repo) = repo();

        assert!(repo.delete(WORKING).is_ok());
    }

    #[test]
    fn two_branches_do_not_see_each_others_versions() {
        let dir = tempfile::tempdir().unwrap();
        let mine = JsonMapRepository::new(Store::new(dir.path(), "feature/x"));
        let theirs = JsonMapRepository::new(Store::new(dir.path(), "feature/y"));
        mine.save(&mapped("abc123")).unwrap();

        assert!(theirs.load_at("abc123").unwrap().is_none());
        assert!(theirs.stored_shas().unwrap().is_empty());
    }
}
