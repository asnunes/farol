use crate::comments::domain::{
    Comment, CommentError, Readiness, RemoteComment, Review, ReviewPublisher,
};
use crate::error::Result;

/// A repository with nowhere to publish to.
///
/// A local repository with no remote is a perfectly good thing to review — it
/// is only publishing that has no meaning there. Standing in for the host keeps
/// that fact in one place, instead of an `Option` every caller has to unwrap
/// and a screen that has to guess what a missing publisher means.
pub struct Unhosted;

impl ReviewPublisher for Unhosted {
    fn readiness(&self, _branch: &str, _head: &str) -> Result<Readiness> {
        Ok(Readiness::NoRemote)
    }

    fn add_pending(&self, _pull: &str, _head: &str, _at: &Comment, _body: &str) -> Result<String> {
        Err(CommentError::NoRemote.into())
    }

    fn edit_pending(&self, _id: &str, _body: &str) -> Result<()> {
        Err(CommentError::NoRemote.into())
    }

    fn remove_pending(&self, _id: &str) -> Result<()> {
        Err(CommentError::NoRemote.into())
    }

    fn remote(&self, _ids: &[String]) -> Result<Vec<RemoteComment>> {
        Err(CommentError::NoRemote.into())
    }

    fn submit(&self, _review: &Review) -> Result<String> {
        Err(CommentError::NoRemote.into())
    }

    fn mark_read(&self, _pull: &str, _paths: &[String]) -> Result<usize> {
        Err(CommentError::NoRemote.into())
    }
}
