use crate::diff::application::{CommitHistory, ReviewScope};
use crate::error::{Error, Result};
use crate::map::application::MapVersions;
use crate::map::domain::{MapRepository, ReviewMap, WORKING};
use std::sync::Arc;

/// Write the map out as one file, and read one back.
///
/// The map is the only thing that travels, and the reason is who writes it.
/// This file is made by the person *asking* for the review, and the map is the
/// whole of what they have to say: the order to read in, and why. Comments are
/// the answer coming back, written by whoever is reviewing, and they will go
/// their own way — to the pull request, where the conversation already lives.
/// One file carrying both would be two use cases pretending to be one.
///
/// The code does not travel either, because the receiver pulls it from git like
/// everybody else. Nor does what anyone has read: importing that would strike
/// half a review through for a person who has opened none of it.
#[derive(Clone)]
pub struct ShareMap {
    versions: MapVersions,
    scope: ReviewScope,
    history: CommitHistory,
    repo: Arc<dyn MapRepository>,
}

impl ShareMap {
    pub fn new(
        versions: MapVersions,
        scope: ReviewScope,
        history: CommitHistory,
        repo: Arc<dyn MapRepository>,
    ) -> Self {
        Self {
            versions,
            scope,
            history,
            repo,
        }
    }

    pub fn export(&self) -> Result<String> {
        let map = self.versions.require_current()?;

        // A map keyed to uncommitted work names a commit nobody else has. It
        // would import as a review of code the other machine cannot fetch.
        if map.generated_at == WORKING {
            return Err(Error::msg(
                "this map was derived against uncommitted work — commit, run `farol map derive`, and export that",
            ));
        }

        let bundle = Bundle { farol: FORMAT, map };
        Ok(serde_json::to_string_pretty(&bundle)? + "\n")
    }

    /// Take a map somebody else wrote and store it as a version here.
    ///
    /// Refused rather than guessed at when it does not belong: a map for
    /// another branch, or one written against a commit this machine has never
    /// seen, would open a review of nothing.
    pub fn import(&self, raw: &str, over: bool) -> Result<ReviewMap> {
        let bundle: Bundle = serde_json::from_str(raw)
            .map_err(|e| Error::msg(format!("this is not a farol export: {e}")))?;
        if bundle.farol != FORMAT {
            return Err(Error::msg(format!(
                "this export is format {}, and this farol reads {FORMAT}",
                bundle.farol
            )));
        }

        let map = bundle.map;
        let here = self.scope.get()?;
        if map.branch != here.branch {
            return Err(Error::msg(format!(
                "this map is for '{}' and you are on '{}' — check that branch out first",
                map.branch, here.branch
            )));
        }
        if !self.history.is_ancestor(&map.generated_at)? {
            return Err(Error::msg(format!(
                "the commit this map was written at, {}, is not here — fetch the branch first",
                &map.generated_at[..map.generated_at.len().min(7)]
            )));
        }
        if !over && self.repo.load_at(&map.generated_at)?.is_some() {
            return Err(Error::msg(format!(
                "there is already a map at {} — pass --over to replace it",
                &map.generated_at[..map.generated_at.len().min(7)]
            )));
        }

        self.repo.save(&map)?;
        Ok(map)
    }
}

/// The envelope: the key names the format and the number versions it, so a file
/// that is not one of ours is refused by the first field rather than by the
/// shape of what is inside.
#[derive(serde::Serialize, serde::Deserialize)]
struct Bundle {
    farol: u32,
    map: ReviewMap,
}

const FORMAT: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services};

    /// A map worth handing over, and the pieces that share it.
    fn sharing(source: FakeDiffSource) -> (ShareMap, Arc<InMemoryMapRepository>) {
        let repo = Arc::new(InMemoryMapRepository::new());
        let handle = Arc::new(source);
        let scope = ReviewScope::new(handle.clone());
        let history = CommitHistory::new(handle);
        let versions = MapVersions::new(scope.clone(), history.clone(), repo.clone());
        (ShareMap::new(versions, scope, history, repo.clone()), repo)
    }

    fn mapped() -> FakeDiffSource {
        FakeDiffSource::with_paths(&["a.rs"])
            .on_commit("abc1234")
            .with_ancestors(&["abc1234"])
    }

    #[test]
    fn what_is_exported_comes_back_as_the_same_map() {
        let (share, _) = sharing(mapped());
        share.export().unwrap_err(); // nothing derived yet

        let (share, repo) = sharing(mapped());
        let svc = services(mapped(), repo.clone());
        svc.editor
            .edit(|_| Ok::<_, crate::map::domain::MapError>(()))
            .unwrap();
        let written = share.export().unwrap();

        // Into a machine that has the commit but no map of its own.
        let (elsewhere, theirs) = sharing(mapped());
        let landed = elsewhere.import(&written, false).unwrap();

        assert_eq!(landed.generated_at, "abc1234");
        assert_eq!(theirs.load_at("abc1234").unwrap().unwrap(), landed);
    }

    #[test]
    fn a_file_that_is_not_an_export_is_refused_by_name() {
        let (share, _) = sharing(mapped());

        let err = share.import("{\"hello\": true}", false).unwrap_err();

        assert!(err.to_string().contains("not a farol export"), "{err}");
    }

    #[test]
    fn a_map_for_another_branch_says_which_one_to_check_out() {
        let (share, repo) = sharing(mapped());
        let svc = services(mapped(), repo.clone());
        svc.editor
            .edit(|_| Ok::<_, crate::map::domain::MapError>(()))
            .unwrap();
        let written = share.export().unwrap().replace("feature/x", "someone/else");

        let err = share.import(&written, false).unwrap_err();

        assert!(err.to_string().contains("someone/else"), "{err}");
        assert!(err.to_string().contains("check that branch out"), "{err}");
    }

    #[test]
    fn a_map_written_against_a_commit_we_do_not_have_says_to_fetch() {
        // The commonest way to receive one: the branch has not been pulled.
        let (share, repo) = sharing(mapped());
        let svc = services(mapped(), repo.clone());
        svc.editor
            .edit(|_| Ok::<_, crate::map::domain::MapError>(()))
            .unwrap();
        let written = share.export().unwrap();

        let (stranger, _) = sharing(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("zzz9999")
                .with_ancestors(&["zzz9999"]),
        );
        let err = stranger.import(&written, false).unwrap_err();

        assert!(err.to_string().contains("fetch the branch"), "{err}");
    }

    #[test]
    fn importing_over_a_map_already_here_is_refused_unless_asked() {
        let (share, repo) = sharing(mapped());
        let svc = services(mapped(), repo.clone());
        svc.editor
            .edit(|_| Ok::<_, crate::map::domain::MapError>(()))
            .unwrap();
        let written = share.export().unwrap();

        let err = share.import(&written, false).unwrap_err();
        assert!(err.to_string().contains("--over"), "{err}");

        assert!(share.import(&written, true).is_ok());
    }
}
