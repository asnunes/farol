use super::{LensAnswer, LensId, LensTarget};
use crate::error::Result;

/// The answers written on this branch.
pub trait LensAnswerRepository: Send + Sync {
    fn all(&self) -> Result<Vec<LensAnswer>>;
    fn find(&self, lens: &LensId, target: &LensTarget) -> Result<Option<LensAnswer>>;
    fn write(&self, answer: &LensAnswer) -> Result<()>;
}
