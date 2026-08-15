use std::sync::Arc;

use crate::diff::application::ReviewScope;
use crate::error::{Error, Result};
use crate::map::application::MapVersions;
use crate::map::domain::{MapRepository, ReviewMap, WORKING};
use crate::map::presentation::short;

/// Hand the map to another machine, and take one that was handed over.
///
/// The map is the only thing that travels, and who writes it is the reason.
/// This file is made by the person *asking* for the review, and the map is the
/// whole of what they have to say: the order to read in, and why. Comments are
/// the answer coming back, written by whoever is reviewing, and they go their
/// own way. The code does not travel either — the other machine pulls it from
/// git — and neither does what anyone has read.
#[derive(Clone)]
pub struct ShareMap {
    versions: MapVersions,
    scope: ReviewScope,
    repo: Arc<dyn MapRepository>,
    /// What this repository is called, for the file to carry.
    name: String,
}

impl ShareMap {
    pub fn new(
        versions: MapVersions,
        scope: ReviewScope,
        repo: Arc<dyn MapRepository>,
        name: String,
    ) -> Self {
        Self {
            versions,
            scope,
            repo,
            name,
        }
    }

    pub fn export(&self) -> Result<Export> {
        let mut map = self.versions.require_current()?;

        // A map keyed to uncommitted work names a commit nobody else has, and
        // would import as a review of code the other machine cannot fetch.
        if map.generated_at == WORKING {
            return Err(Error::msg(
                "this map was derived against uncommitted work — commit, run `farol map derive`, and export that",
            ));
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

    /// Store a map somebody else exported as a version here.
    ///
    /// Only the base commit is checked. Two repositories that share it are the
    /// same repository for every purpose farol has, while the name of the
    /// branch is not: `gh pr checkout` renames it half the time. A head that
    /// differs is reported rather than refused — being a few commits ahead of
    /// the map is the ordinary state of a review, and the screen already says
    /// how far.
    pub fn import(&self, raw: &str) -> Result<Import> {
        let bundle: Bundle = serde_json::from_str(raw)
            .map_err(|e| Error::msg(format!("this is not a farol export: {e}")))?;

        let here = self.scope.get()?;
        if bundle.base != here.base_sha {
            return Err(Error::msg(format!(
                "this map was written against base {}, and here the base is {} — it belongs to another repository",
                short(&bundle.base),
                short(&here.base_sha)
            )));
        }

        let behind = match bundle.head == here.head_sha {
            true => None,
            false => Some((bundle.head.clone(), here.head_sha.clone())),
        };

        self.repo.save(&bundle.map)?;
        Ok(Import {
            map: bundle.map,
            behind,
        })
    }
}

/// The file, and what to call it.
#[derive(Debug)]
pub struct Export {
    pub file: String,
    pub body: String,
}

/// What landed, and the heads that did not match if they did not.
#[derive(Debug)]
pub struct Import {
    pub map: ReviewMap,
    /// The commit the map was written at, and the one we are on.
    pub behind: Option<(String, String)>,
}

/// The format. There is no version of its own: the map carries the only one
/// there is, and a second number beside it would be a second thing to keep in
/// step. A file that is not one of these fails to deserialise on a missing
/// field, which is refusal enough.
#[derive(serde::Serialize, serde::Deserialize)]
struct Bundle {
    /// The two labels. Neither is compared — they are here to be read.
    repo: String,
    branch: String,
    /// What identifies the review: shared base means shared repository.
    base: String,
    /// Where the map was written. Reported when it differs, never refused.
    head: String,
    map: ReviewMap,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::{LineRange, MapError};
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, orphaned, services};

    /// A repository with a map worth handing over, and the pieces that hand it.
    fn sharing(review: fn() -> FakeDiffSource) -> (ShareMap, crate::testing::MapServices) {
        let repo = Arc::new(InMemoryMapRepository::new());
        let scope = ReviewScope::new(Arc::new(review()));
        let svc = services(review(), repo.clone());
        let share = ShareMap::new(svc.versions.clone(), scope, repo, "asnunes/farol".into());
        (share, svc)
    }

    fn abc() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .from_base("base1234")
            .on_commit("abc1234")
            .with_ancestors(&["abc1234"])
    }

    /// The same review, a few commits further on.
    fn def() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .from_base("base1234")
            .on_commit("def5678")
            .with_ancestors(&["def5678"])
    }

    /// Same commit, another repository: the base is what differs.
    fn other_repo() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .from_base("zzz9999")
            .on_commit("abc1234")
            .with_ancestors(&["abc1234"])
    }

    fn abc_dirty() -> FakeDiffSource {
        abc().dirty()
    }

    fn started(svc: &crate::testing::MapServices) {
        svc.editor.edit(|_| Ok::<_, MapError>(())).unwrap();
    }

    /// A block to hang an orphaned note off, since a note needs one.
    fn with_a_block(svc: &crate::testing::MapServices, scope: &ReviewScope) {
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

    #[test]
    fn a_map_handed_over_arrives_as_it_left() {
        let (mine, svc) = sharing(abc);
        started(&svc);
        let export = mine.export().unwrap();

        // Another machine on the same base, with nothing of its own.
        let (theirs, elsewhere) = sharing(abc);
        let landed = theirs.import(&export.body).unwrap();

        assert_eq!(landed.map.generated_at, "abc1234");
        assert!(landed.behind.is_none());
        assert_eq!(
            elsewhere.versions.current().unwrap().unwrap(),
            landed.map,
            "it has to be stored, not only returned"
        );
    }

    #[test]
    fn the_file_is_named_after_the_commit_it_describes() {
        let (mine, svc) = sharing(abc);
        started(&svc);

        assert_eq!(mine.export().unwrap().file, "map-abc1234.farol.json");
    }

    #[test]
    fn a_map_from_another_repository_is_refused_by_its_base() {
        // Shared base is what makes two clones the same review. Nothing else
        // is compared, so this is the only thing standing between a map and
        // the wrong repository.
        let (mine, svc) = sharing(abc);
        started(&svc);
        let export = mine.export().unwrap();

        let (stranger, _) = sharing(other_repo);
        let err = stranger.import(&export.body).unwrap_err();

        assert!(err.to_string().contains("another repository"), "{err}");
    }

    #[test]
    fn a_head_that_moved_on_is_reported_rather_than_refused() {
        // The ordinary state of a review: the branch grew after the map was
        // written, and the screen already says how far behind it is.
        let (mine, svc) = sharing(abc);
        started(&svc);
        let export = mine.export().unwrap();

        let (ahead, _) = sharing(def);
        let landed = ahead.import(&export.body).unwrap();

        assert_eq!(
            landed.behind,
            Some(("abc1234".to_string(), "def5678".to_string()))
        );
        assert_eq!(landed.map.generated_at, "abc1234");
    }

    #[test]
    fn importing_writes_over_whatever_was_there() {
        // Whoever asked for the review owns the map; the reader is receiving a
        // newer copy of it, which is the whole point of importing twice.
        let (mine, svc) = sharing(abc);
        started(&svc);
        let export = mine.export().unwrap();

        let (theirs, _) = sharing(abc);
        theirs.import(&export.body).unwrap();

        assert!(theirs.import(&export.body).is_ok());
    }

    #[test]
    fn a_note_still_waiting_for_a_decision_does_not_travel() {
        // It is a decision for whoever wrote the note. Sent along, it arrives
        // as a decision for somebody who cannot make it.
        let (mine, svc) = sharing(abc);
        started(&svc);
        with_a_block(&svc, &ReviewScope::new(Arc::new(abc())));
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

        let export = mine.export().unwrap();

        assert!(!export.body.contains("expensive prose"), "{}", export.body);
        let (theirs, _) = sharing(abc);
        assert!(
            theirs
                .import(&export.body)
                .unwrap()
                .map
                .orphans()
                .is_empty()
        );
    }

    #[test]
    fn a_map_derived_over_uncommitted_work_cannot_be_handed_over() {
        // It names a commit nobody else can fetch.
        let (mine, svc) = sharing(abc_dirty);
        started(&svc);

        let err = mine.export().unwrap_err();

        assert!(err.to_string().contains("uncommitted work"), "{err}");
    }
}
