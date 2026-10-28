use std::sync::Arc;

use crate::diff::application::ReviewScope;
use crate::error::Result;
use crate::lenses::application::covered_now;
use crate::lenses::domain::{
    Freshness, LensAnswerRepository, LensCatalogRepository, LensDefinition, LensGroup, LensTarget,
};
use crate::map::application::MapVersions;
use crate::map::domain::Slug;

/// What the session still has to write: every lens switched on, on every
/// target it applies to, that has no answer or whose answer is Outdated.
///
/// A file lens is listed for every file in the review, because only the
/// session can tell which files it applies to. An empty answer is how it says
/// a file does not need one, and that answer stays until the file changes.
#[derive(Clone)]
pub struct PendingAnswers {
    catalog: Arc<dyn LensCatalogRepository>,
    answers: Arc<dyn LensAnswerRepository>,
    scope: ReviewScope,
    versions: MapVersions,
}

impl PendingAnswers {
    pub fn new(
        catalog: Arc<dyn LensCatalogRepository>,
        answers: Arc<dyn LensAnswerRepository>,
        scope: ReviewScope,
        versions: MapVersions,
    ) -> Self {
        Self {
            catalog,
            answers,
            scope,
            versions,
        }
    }

    pub fn execute(&self) -> Result<Vec<Pending>> {
        let lenses = self.catalog.read()?.enabled();
        let map = self.versions.require_current()?;
        let scope = self.scope.get()?;
        let blocks: Vec<Slug> = map
            .blocks()
            .iter()
            .filter(|b| map.paths_in(&b.slug).any(|p| scope.contains(p)))
            .map(|b| b.slug.clone())
            .collect();

        let mut out = Vec::new();
        for lens in lenses {
            let targets: Vec<LensTarget> = match lens.group {
                LensGroup::Pr => vec![LensTarget::Pr],
                LensGroup::Block => blocks
                    .iter()
                    .map(|slug| LensTarget::Block { slug: slug.clone() })
                    .collect(),
                LensGroup::File => scope
                    .files
                    .iter()
                    .map(|f| LensTarget::File {
                        path: f.path.clone(),
                    })
                    .collect(),
            };
            for target in targets {
                let covers = covered_now(&target, &map, scope);
                let why = match self.answers.find(&lens.id, &target)? {
                    None => Some(Why::Missing),
                    Some(answer) if answer.target != target => Some(Why::Missing),
                    Some(answer) => match answer.freshness(&covers, &lens.prompt_hash()) {
                        Freshness::Written => None,
                        Freshness::Outdated { .. } => Some(Why::Outdated),
                    },
                };
                if let Some(why) = why {
                    out.push(Pending {
                        lens: lens.clone(),
                        target,
                        why,
                    });
                }
            }
        }
        Ok(out)
    }
}

/// One answer the session owes.
pub struct Pending {
    pub lens: LensDefinition,
    pub target: LensTarget,
    pub why: Why,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Missing,
    Outdated,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lenses::application::{Aim, WriteAnswer};
    use crate::lenses::domain::LensId;
    use crate::testing::{InMemoryLensAnswers, InMemoryLensCatalog, with_block};

    #[test]
    fn a_written_answer_leaves_the_list_and_every_file_is_offered_to_a_file_lens() {
        let (svc, scope) = with_block(&["a.rs", "b.rs"]);
        let catalog = Arc::new(InMemoryLensCatalog::default());
        let answers = Arc::new(InMemoryLensAnswers::default());
        let pending = PendingAnswers::new(
            catalog.clone(),
            answers.clone(),
            scope.clone(),
            svc.versions.clone(),
        );
        let before = pending.execute().unwrap();
        // overview, risk, evidence once; summary for the block; tests and smell for each of the two files.
        assert_eq!(before.len(), 8);

        WriteAnswer::new(
            catalog,
            answers,
            scope.clone(),
            crate::diff::application::FileDiffs::new(Arc::new(
                crate::testing::FakeDiffSource::with_paths(&["a.rs", "b.rs"]),
            )),
            svc.versions,
        )
        .execute(
            &LensId::parse("tests").unwrap(),
            Aim::File(scope.path("a.rs").unwrap()),
            String::new().into(),
        )
        .unwrap();

        let after = pending.execute().unwrap();
        assert_eq!(
            after.len(),
            7,
            "an empty answer says the lens does not apply, and is not asked again"
        );
        assert!(after.iter().all(|p| p.why == Why::Missing));
    }
}
