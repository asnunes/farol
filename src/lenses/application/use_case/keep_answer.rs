use std::sync::Arc;

use crate::diff::application::ReviewScope;
use crate::error::Result;
use crate::lenses::application::{Aim, ShowAnswer, WriteAnswer};
use crate::lenses::domain::{LensAnswer, LensAnswerRepository, LensError, LensId};

/// Keep what a lens said, against the review as it is now.
///
/// After a commit the session judges each outdated answer: one that still
/// holds is kept rather than rewritten, which costs it nothing and tells the
/// reader it was looked at. It goes through the same checks as a write — the
/// citations are held to the new code, and one that no longer lands refuses
/// the keep, which is the sign the answer needs rewriting.
#[derive(Clone)]
pub struct KeepAnswer {
    show: ShowAnswer,
    write: WriteAnswer,
    scope: ReviewScope,
}

impl KeepAnswer {
    pub fn new(
        answers: Arc<dyn LensAnswerRepository>,
        write: WriteAnswer,
        scope: ReviewScope,
    ) -> Self {
        Self {
            show: ShowAnswer::new(answers),
            write,
            scope,
        }
    }

    pub fn execute(&self, id: &LensId, aim: Aim) -> Result<LensAnswer> {
        let Some(answer) = self.show.execute(id, aim.clone())? else {
            return Err(LensError::NothingToKeep { id: id.to_string() }.into());
        };
        // A file it asked a person to read that left the review is dropped.
        let read = answer
            .read
            .iter()
            .filter_map(|path| self.scope.path(path).ok())
            .collect();
        self.write.execute(id, aim, answer.text, read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::application::FileDiffs;
    use crate::lenses::domain::{Covered, LensTarget};
    use crate::testing::{FakeDiffSource, InMemoryLensAnswers, InMemoryLensCatalog, with_block};

    #[test]
    fn a_kept_answer_keeps_its_words_and_takes_the_review_as_it_is_now() {
        let (svc, scope) = with_block(&["a.rs"]);
        let answers = Arc::new(InMemoryLensAnswers::default());
        let write = WriteAnswer::new(
            Arc::new(InMemoryLensCatalog::default()),
            answers.clone(),
            scope.clone(),
            FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&["a.rs"]))),
            svc.versions,
        );
        let id = LensId::parse("overview").unwrap();
        let mut old = write
            .execute(&id, Aim::Pr, "It retries.".into(), vec![])
            .unwrap();
        old.covers = vec![Covered {
            path: "a.rs".into(),
            blob: "before".into(),
        }];
        answers.write(&old).unwrap();

        let kept = KeepAnswer::new(answers, write, scope)
            .execute(&id, Aim::Pr)
            .unwrap();

        assert_eq!(kept.text, "It retries.");
        assert_ne!(
            kept.covers, old.covers,
            "kept against the files as they are now"
        );
        assert_eq!(kept.target, LensTarget::Pr);
    }

    #[test]
    fn nothing_written_is_nothing_to_keep() {
        let (svc, scope) = with_block(&["a.rs"]);
        let answers = Arc::new(InMemoryLensAnswers::default());
        let write = WriteAnswer::new(
            Arc::new(InMemoryLensCatalog::default()),
            answers.clone(),
            scope.clone(),
            FileDiffs::new(Arc::new(FakeDiffSource::with_paths(&["a.rs"]))),
            svc.versions,
        );

        let error = KeepAnswer::new(answers, write, scope)
            .execute(&LensId::parse("risk").unwrap(), Aim::Pr)
            .unwrap_err()
            .to_string();

        assert!(error.contains("farol lens write"), "{error}");
    }
}
