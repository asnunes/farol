use crate::diff::application::ReviewScope;
use crate::error::Result;
use crate::map::application::{Bundle, MapVersions};
use crate::map::domain::{MapError, WORKING};
use crate::map::presentation::short;

/// Write the map out for another machine to read.
///
/// The map is the only thing that goes, and who writes the file is the reason:
/// it is made by the person *asking* for the review, and the map is the whole
/// of what they have to say. Comments are the answer coming back, written by
/// whoever is reviewing, and they go their own way. The code does not travel
/// either — the other machine pulls it from git — and neither does what anyone
/// has read.
#[derive(Clone)]
pub struct ExportMap {
    versions: MapVersions,
    scope: ReviewScope,
    /// What this repository is called, for the file to carry.
    name: String,
}

impl ExportMap {
    pub fn new(versions: MapVersions, scope: ReviewScope, name: String) -> Self {
        Self {
            versions,
            scope,
            name,
        }
    }

    pub fn execute(&self) -> Result<Export> {
        let mut map = self.versions.require_current()?;

        // A map keyed to uncommitted work names a commit nobody else has, and
        // would import as a review of code the other machine cannot fetch.
        if map.generated_at == WORKING {
            return Err(MapError::ExportsUncommitted.into());
        }

        // A deactivated note is a decision waiting for whoever wrote it. Sent
        // along, it would arrive as a decision for somebody who cannot make it:
        // they did not write the note and cannot know whether it still holds.
        map.clear_orphans();

        let scope = self.scope.get()?;
        let file = format!("map-{}.farol.json", short(&map.generated_at));
        let bundle = Bundle {
            repo: self.name.clone(),
            branch: map.branch.clone(),
            base: scope.base_sha.clone(),
            head: map.generated_at.clone(),
            map,
        };

        Ok(Export {
            file,
            body: serde_json::to_string_pretty(&bundle)? + "\n",
        })
    }
}

/// The file, and what to call it.
#[derive(Debug)]
pub struct Export {
    pub file: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::{LineRange, MapError};
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, orphaned, services};
    use std::sync::Arc;

    /// A review with a map worth handing over, and the exporter for it.
    fn exporting(review: fn() -> FakeDiffSource) -> (ExportMap, crate::testing::MapServices) {
        let repo = Arc::new(InMemoryMapRepository::new());
        let scope = ReviewScope::new(Arc::new(review()));
        let svc = services(review(), repo);
        let export = ExportMap::new(svc.versions.clone(), scope, "asnunes/farol".into());
        (export, svc)
    }

    fn abc() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .from_base("base1234")
            .on_commit("abc1234")
            .with_ancestors(&["abc1234"])
    }

    fn abc_dirty() -> FakeDiffSource {
        abc().dirty()
    }

    fn started(svc: &crate::testing::MapServices) {
        svc.editor.edit(|_| Ok::<_, MapError>(())).unwrap();
    }

    #[test]
    fn the_file_carries_the_map_and_the_two_commits() {
        let (export, svc) = exporting(abc);
        started(&svc);

        let out = export.execute().unwrap();

        let bundle: Bundle = serde_json::from_str(&out.body).unwrap();
        assert_eq!(bundle.repo, "asnunes/farol");
        assert_eq!(bundle.base, "base1234");
        assert_eq!(bundle.head, "abc1234");
        assert_eq!(bundle.map.generated_at, "abc1234");
    }

    #[test]
    fn the_file_is_named_after_the_commit_it_describes() {
        let (export, svc) = exporting(abc);
        started(&svc);

        assert_eq!(export.execute().unwrap().file, "map-abc1234.farol.json");
    }

    #[test]
    fn a_note_still_waiting_for_a_decision_does_not_travel() {
        // It is a decision for whoever wrote the note. Sent along, it arrives
        // as a decision for somebody who cannot make it.
        let (export, svc) = exporting(abc);
        started(&svc);
        with_a_block(&svc);
        orphaned(
            &svc.editor,
            LineRange::new(10, 12).unwrap(),
            "expensive prose",
        );
        assert!(
            !svc.versions
                .current()
                .unwrap()
                .unwrap()
                .orphans()
                .is_empty()
        );

        let out = export.execute().unwrap();

        assert!(!out.body.contains("expensive prose"), "{}", out.body);
    }

    #[test]
    fn a_map_derived_over_uncommitted_work_cannot_be_handed_over() {
        // It names a commit nobody else can fetch.
        let (export, svc) = exporting(abc_dirty);
        started(&svc);

        let err = export.execute().unwrap_err();

        assert!(err.to_string().contains("uncommitted work"), "{err}");
    }

    /// A block to hang an orphaned note off, since a note needs one.
    fn with_a_block(svc: &crate::testing::MapServices) {
        let scope = ReviewScope::new(Arc::new(abc()));
        let paths = scope.paths(&["a.rs".to_string()]).unwrap();
        crate::map::application::AddBlock::new(svc.editor.clone())
            .execute(
                &crate::testing::slug("core"),
                "t",
                "c",
                crate::map::domain::Position::End,
                &paths,
            )
            .unwrap();
    }
}
