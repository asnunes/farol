use std::sync::Arc;

use crate::error::Result;
use crate::lenses::application::Aim;
use crate::lenses::domain::{LensAnswer, LensAnswerRepository, LensId, LensTarget};

/// What a lens says about one target, for the session to read back.
#[derive(Clone)]
pub struct ShowAnswer {
    answers: Arc<dyn LensAnswerRepository>,
}

impl ShowAnswer {
    pub fn new(answers: Arc<dyn LensAnswerRepository>) -> Self {
        Self { answers }
    }

    pub fn execute(&self, id: &LensId, aim: Aim) -> Result<Option<LensAnswer>> {
        let target = match aim {
            Aim::Pr => LensTarget::Pr,
            Aim::Block(slug) => LensTarget::Block { slug },
            Aim::File(path) => LensTarget::File {
                path: path.as_str().to_string(),
            },
        };
        Ok(self
            .answers
            .find(id, &target)?
            .filter(|a| a.target == target))
    }
}
