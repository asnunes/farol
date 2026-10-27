use std::sync::Arc;

use crate::error::Result;
use crate::lenses::application::{LensReading, covered_now};
use crate::lenses::domain::{LensAnswerRepository, LensCatalogRepository};
use crate::map::application::{GetReview, ReviewSnapshot};

/// The review the screen draws, with what its lenses say and how each answer
/// stands against the review as it is now.
///
/// It wraps the map's own read rather than adding to it: the map knows nothing
/// of lenses, and the lenses need the map, so the dependency points one way.
#[derive(Clone)]
pub struct GetReviewWithLenses {
    review: GetReview,
    catalog: Arc<dyn LensCatalogRepository>,
    answers: Arc<dyn LensAnswerRepository>,
}

impl GetReviewWithLenses {
    pub fn new(
        review: GetReview,
        catalog: Arc<dyn LensCatalogRepository>,
        answers: Arc<dyn LensAnswerRepository>,
    ) -> Self {
        Self {
            review,
            catalog,
            answers,
        }
    }

    pub fn execute(&self) -> Result<(ReviewSnapshot, LensReading)> {
        let snapshot = self.review.execute()?;
        let lenses = self.catalog.read()?.enabled();
        // Answers of a lens switched off or deleted are kept on disk and left
        // off the screen, so switching it back on brings them back.
        let answers = self
            .answers
            .all()?
            .into_iter()
            .filter_map(|answer| {
                let lens = lenses.iter().find(|l| l.id == answer.lens)?;
                let now = covered_now(&answer.target, &snapshot.map, &snapshot.scope);
                let freshness = answer.freshness(&now, &lens.prompt_hash());
                Some((answer, freshness))
            })
            .collect();
        Ok((snapshot, LensReading { lenses, answers }))
    }
}
