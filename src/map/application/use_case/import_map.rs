use std::sync::Arc;

use crate::diff::application::ReviewScope;
use crate::error::Result;
use crate::map::application::Bundle;
use crate::map::domain::{MapError, MapRepository, ReviewMap};
use crate::shared::short;

/// Store a map somebody else exported as a version here.
///
/// Only the base commit is checked. Two repositories that share it are the same
/// repository for every purpose farol has, while the name of the branch is not:
/// `gh pr checkout` renames it half the time. A head that differs is reported
/// rather than refused — being a few commits ahead of the map is the ordinary
/// state of a review, and the screen already says how far.
#[derive(Clone)]
pub struct ImportMap {
    scope: ReviewScope,
    repo: Arc<dyn MapRepository>,
}

impl ImportMap {
    pub fn new(scope: ReviewScope, repo: Arc<dyn MapRepository>) -> Self {
        Self { scope, repo }
    }

    pub fn execute(&self, raw: &str) -> Result<Import> {
        let bundle = Bundle::parse(raw)?;

        let here = self.scope.get()?;
        if bundle.base != here.base_sha {
            return Err(MapError::ForeignBase {
                theirs: short(&bundle.base).to_string(),
                ours: short(&here.base_sha).to_string(),
            }
            .into());
        }

        let behind = match bundle.head == here.head_sha {
            true => None,
            false => Some((bundle.head.clone(), here.head_sha.clone())),
        };

        // Straight over whatever is stored for that commit. Whoever asked for
        // the review owns the map, and the reader receiving a newer copy is the
        // whole point of importing twice.
        self.repo.save(&bundle.map)?;
        Ok(Import {
            map: bundle.map,
            behind,
        })
    }
}

/// What landed, and the two heads when they did not match.
#[derive(Debug)]
pub struct Import {
    pub map: ReviewMap,
    /// The commit the map was written at, and the one we are on.
    pub behind: Option<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::ExportMap;
    use crate::map::domain::MapError;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services};

    /// The importer, and the store it writes into.
    fn importing(review: fn() -> FakeDiffSource) -> (ImportMap, Arc<InMemoryMapRepository>) {
        let repo = Arc::new(InMemoryMapRepository::new());
        let scope = ReviewScope::new(Arc::new(review()));
        (ImportMap::new(scope, repo.clone()), repo)
    }

    /// A file as another machine would have written it.
    fn handed_over(review: fn() -> FakeDiffSource) -> String {
        let repo = Arc::new(InMemoryMapRepository::new());
        let svc = services(review(), repo);
        svc.editor.edit(|_| Ok::<_, MapError>(())).unwrap();
        ExportMap::new(
            svc.versions,
            ReviewScope::new(Arc::new(review())),
            "asnunes/farol".into(),
        )
        .execute()
        .unwrap()
        .body
    }

    fn abc() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .from_base("base1234")
            .on_commit("abc1234000000000000000000000000000000000")
            .with_ancestors(&["abc1234000000000000000000000000000000000"])
    }

    /// The same review, further on.
    fn def() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .from_base("base1234")
            .on_commit("def5678000000000000000000000000000000000")
            .with_ancestors(&["def5678000000000000000000000000000000000"])
    }

    /// Another repository: the base is what differs.
    fn other_repo() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .from_base("zzz9999")
            .on_commit("abc1234000000000000000000000000000000000")
            .with_ancestors(&["abc1234000000000000000000000000000000000"])
    }

    #[test]
    fn a_map_handed_over_arrives_as_it_left() {
        let (import, repo) = importing(abc);

        let landed = import.execute(&handed_over(abc)).unwrap();

        assert_eq!(
            landed.map.generated_at,
            "abc1234000000000000000000000000000000000"
        );
        assert!(landed.behind.is_none());
        assert_eq!(
            repo.load_at("abc1234000000000000000000000000000000000")
                .unwrap()
                .unwrap(),
            landed.map,
            "it has to be stored, not only returned"
        );
    }

    #[test]
    fn a_map_from_another_repository_is_refused_by_its_base() {
        // Shared base is what makes two clones the same review, and nothing
        // else is compared — so this is all that stands between a map and the
        // wrong repository.
        let (import, _) = importing(other_repo);

        let err = import.execute(&handed_over(abc)).unwrap_err();

        assert!(err.to_string().contains("another repository"), "{err}");
    }

    #[test]
    fn a_head_that_moved_on_is_reported_rather_than_refused() {
        // The ordinary state of a review: the branch grew after the map was
        // written, and the screen already says how far behind it is.
        let (import, _) = importing(def);

        let landed = import.execute(&handed_over(abc)).unwrap();

        assert_eq!(
            landed.behind,
            Some((
                "abc1234000000000000000000000000000000000".to_string(),
                "def5678000000000000000000000000000000000".to_string()
            ))
        );
        assert_eq!(
            landed.map.generated_at,
            "abc1234000000000000000000000000000000000"
        );
    }

    #[test]
    fn importing_writes_over_whatever_was_there() {
        // Whoever asked for the review owns the map; the reader is receiving a
        // newer copy, which is the whole point of importing twice.
        let (import, _) = importing(abc);
        import.execute(&handed_over(abc)).unwrap();

        assert!(import.execute(&handed_over(abc)).is_ok());
    }

    #[test]
    fn an_invalid_import_never_reaches_storage() {
        let (import, repo) = importing(abc);
        let mut bundle: serde_json::Value = serde_json::from_str(&handed_over(abc)).unwrap();
        bundle["map"]["generated_at"] = "../../outside".into();

        let error = import.execute(&bundle.to_string()).unwrap_err();

        assert!(error.to_string().contains("map.generated_at"), "{error}");
        assert!(
            repo.stored_shas().unwrap().is_empty(),
            "invalid input must be rejected before calling the repository"
        );
    }

    #[test]
    fn a_file_that_is_not_an_export_is_refused_by_name() {
        let (import, _) = importing(abc);

        let err = import.execute("{\"hello\": true}").unwrap_err();

        assert!(err.to_string().contains("not a farol export"), "{err}");
    }
}
