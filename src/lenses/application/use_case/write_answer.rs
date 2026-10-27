use std::sync::Arc;

use crate::diff::application::{FileDiffs, ReviewScope};
use crate::diff::domain::{ReviewPath, Side};
use crate::error::Result;
use crate::lenses::application::{Aim, covered_now};
use crate::lenses::domain::{
    Citation, LensAnswer, LensAnswerRepository, LensCatalogRepository, LensError, LensGroup,
    LensId, LensTarget,
};
use crate::map::application::MapVersions;
use crate::map::domain::MapError;
use crate::shared::WORKING;

/// Write what a lens says about its target, replacing what it said before.
///
/// Everything an answer needs to be held to later is recorded here, once: the
/// files it covers as they are now, the prompt it followed, and the citations
/// it makes, each checked against the review. A citation of a line that does
/// not exist is refused, because an answer pointing at code that is not there
/// is the one mistake the reader cannot catch from the screen.
#[derive(Clone)]
pub struct WriteAnswer {
    catalog: Arc<dyn LensCatalogRepository>,
    answers: Arc<dyn LensAnswerRepository>,
    scope: ReviewScope,
    diffs: FileDiffs,
    versions: MapVersions,
}

impl WriteAnswer {
    pub fn new(
        catalog: Arc<dyn LensCatalogRepository>,
        answers: Arc<dyn LensAnswerRepository>,
        scope: ReviewScope,
        diffs: FileDiffs,
        versions: MapVersions,
    ) -> Self {
        Self {
            catalog,
            answers,
            scope,
            diffs,
            versions,
        }
    }

    pub fn execute(
        &self,
        id: &LensId,
        aim: Aim,
        text: String,
        read: Vec<ReviewPath>,
    ) -> Result<LensAnswer> {
        let lens = self.catalog.read()?.require(id)?.definition;
        let map = self.versions.require_current()?;
        let target = match (lens.group, aim) {
            (LensGroup::Pr, Aim::Pr) => LensTarget::Pr,
            (LensGroup::Block, Aim::Block(slug)) => {
                if map.block(&slug).is_none() {
                    return Err(MapError::UnknownBlock {
                        slug: slug.to_string(),
                        existing: map.slugs(),
                    }
                    .into());
                }
                LensTarget::Block { slug }
            }
            (LensGroup::File, Aim::File(path)) => LensTarget::File {
                path: path.as_str().to_string(),
            },
            (group, _) => return Err(wrong_target(id, group).into()),
        };
        if let Some(there) = self.answers.find(id, &target)?
            && there.target != target
        {
            return Err(LensError::Collision {
                written: there.target.to_string(),
                asked: target.to_string(),
            }
            .into());
        }
        let scope = self.scope.get()?;
        let answer = LensAnswer {
            lens: lens.id.clone(),
            covers: covered_now(&target, &map, scope),
            target,
            citations: self.check(&text)?,
            text,
            written_at: if scope.dirty {
                WORKING.to_string()
            } else {
                scope.head_sha.clone()
            },
            prompt_hash: lens.prompt_hash(),
            read: read.iter().map(|p| p.as_str().to_string()).collect(),
        };
        self.answers.write(&answer)?;
        Ok(answer)
    }

    /// Each citation checked against the review. One on the new side is held to
    /// the file's length; one on the old side is linked when the diff prints
    /// it. What cannot be checked — a file outside the review, an old line
    /// outside the hunks — is kept as text, not refused.
    fn check(&self, text: &str) -> Result<Vec<Citation>> {
        let mut out = Vec::new();
        for mut citation in Citation::all_in(text) {
            if let Ok(path) = self.scope.path(&citation.path) {
                citation.linked = match citation.side {
                    Side::New if citation.to > path.lines() => {
                        return Err(LensError::PastTheEnd {
                            citation: citation.text.trim().to_string(),
                            path: citation.path,
                            lines: path.lines(),
                        }
                        .into());
                    }
                    Side::New => true,
                    Side::Old => self
                        .diffs
                        .of(path.as_str())
                        .map(|diff| diff.shows(Side::Old, citation.from, citation.to))
                        .unwrap_or(false),
                };
            }
            out.push(citation);
        }
        Ok(out)
    }
}

fn wrong_target(id: &LensId, group: LensGroup) -> LensError {
    let (wants, hint) = match group {
        LensGroup::Pr => ("once for the pull request", "Drop --block and --file."),
        LensGroup::Block => (
            "once per block",
            "Pass --block <slug>; farol map show lists the blocks.",
        ),
        LensGroup::File => (
            "once per file",
            "Pass --file <path>; farol scope lists the files.",
        ),
    };
    LensError::WrongTarget {
        id: id.to_string(),
        wants,
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{InMemoryLensAnswers, InMemoryLensCatalog, slug, with_block};

    fn setup() -> (WriteAnswer, Arc<InMemoryLensAnswers>, ReviewScope) {
        let (svc, scope) = with_block(&["a.rs", "b.rs"]);
        let source = Arc::new(
            crate::testing::FakeDiffSource::with_paths(&["a.rs", "b.rs"])
                .with_line_count("a.rs", 100)
                .with_line_count("b.rs", 100),
        );
        let answers = Arc::new(InMemoryLensAnswers::default());
        let write = WriteAnswer::new(
            Arc::new(InMemoryLensCatalog::default()),
            answers.clone(),
            scope.clone(),
            FileDiffs::new(source),
            svc.versions,
        );
        (write, answers, scope)
    }

    fn id(raw: &str) -> LensId {
        LensId::parse(raw).unwrap()
    }

    #[test]
    fn an_answer_remembers_the_files_it_covers_and_the_prompt_it_followed() {
        let (write, answers, scope) = setup();

        let answer = write
            .execute(
                &id("tests"),
                Aim::File(scope.path("a.rs").unwrap()),
                "- retries `a.rs:10`".into(),
                vec![],
            )
            .unwrap();

        assert_eq!(answer.covers.len(), 1);
        assert_eq!(answer.covers[0].path, "a.rs");
        assert!(answer.citations[0].linked);
        assert_eq!(answers.all().unwrap().len(), 1);
    }

    #[test]
    fn a_pr_lens_covers_every_file_and_a_block_lens_its_block() {
        let (write, _, _) = setup();

        let pr = write
            .execute(&id("overview"), Aim::Pr, "x".into(), vec![])
            .unwrap();

        assert_eq!(
            pr.covers
                .iter()
                .map(|c| c.path.as_str())
                .collect::<Vec<_>>(),
            vec!["a.rs", "b.rs"]
        );
    }

    #[test]
    fn a_lens_aimed_at_the_wrong_kind_of_target_is_refused_with_the_way_out() {
        let (write, _, scope) = setup();

        let on_a_file = write
            .execute(
                &id("overview"),
                Aim::File(scope.path("a.rs").unwrap()),
                "x".into(),
                vec![],
            )
            .unwrap_err()
            .to_string();
        let on_the_pr = write
            .execute(&id("tests"), Aim::Pr, "x".into(), vec![])
            .unwrap_err()
            .to_string();

        assert!(on_a_file.contains("Drop --block"), "{on_a_file}");
        assert!(on_the_pr.contains("--file <path>"), "{on_the_pr}");
    }

    #[test]
    fn a_block_that_is_not_on_the_map_is_refused_naming_the_blocks() {
        let mut catalog = crate::lenses::domain::LensCatalog::default();
        catalog.switch(&id("architecture"), true).unwrap();
        let (write, _, _) = setup();
        let write = WriteAnswer {
            catalog: Arc::new(InMemoryLensCatalog::with(catalog)),
            ..write
        };

        let error = write
            .execute(
                &id("architecture"),
                Aim::Block(slug("nowhere")),
                "x".into(),
                vec![],
            )
            .unwrap_err()
            .to_string();

        assert!(error.contains("core"), "{error}");
        assert!(
            write
                .execute(
                    &id("architecture"),
                    Aim::Block(slug("core")),
                    "x".into(),
                    vec![]
                )
                .is_ok()
        );
    }

    #[test]
    fn a_citation_past_the_end_of_the_file_is_refused_and_nothing_is_written() {
        let (write, answers, _) = setup();

        let error = write
            .execute(
                &id("overview"),
                Aim::Pr,
                "see `a.rs:150-160`".into(),
                vec![],
            )
            .unwrap_err()
            .to_string();

        assert!(error.contains("100 lines"), "{error}");
        assert!(answers.all().unwrap().is_empty());
    }

    #[test]
    fn a_citation_that_cannot_be_checked_is_kept_as_text() {
        let (write, _, _) = setup();

        let answer = write
            .execute(
                &id("overview"),
                Aim::Pr,
                "see `AGENTS.md:12` and `a.rs:3 (old)`".into(),
                vec![],
            )
            .unwrap();

        assert!(
            !answer.citations[0].linked,
            "a file outside the review has no link to go to"
        );
        assert_eq!(
            answer.citations[1].side,
            Side::Old,
            "an old line is never held to the new file's length"
        );
    }
}
