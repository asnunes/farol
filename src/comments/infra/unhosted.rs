use crate::comments::domain::{CommentError, Readiness, Review, ReviewPublisher};
use crate::error::Result;

/// A repository with nowhere to publish to.
///
/// A local repository with no remote is a perfectly good thing to review — it
/// is only publishing that has no meaning there. Standing in for the host keeps
/// that fact in one place, instead of an `Option` every caller has to unwrap
/// and a screen that has to guess what a missing publisher means.
pub struct Unhosted;

impl ReviewPublisher for Unhosted {
    fn readiness(&self, _branch: &str) -> Result<Readiness> {
        Ok(Readiness::NoRemote)
    }

    fn publish(&self, _review: &Review) -> Result<String> {
        Err(CommentError::NoRemote.into())
    }

    fn mark_read(&self, _pull: &str, _paths: &[String]) -> Result<usize> {
        Err(CommentError::NoRemote.into())
    }
}
