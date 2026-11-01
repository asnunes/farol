use crate::comments::domain::{CommentError, Readiness, RemoteComment, Review, ReviewPublisher};
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

    fn add_pending(
        &self,
        _pull: &str,
        _head: &str,
        _at: &crate::comments::domain::Lines,
        _body: &str,
    ) -> Result<String> {
        Err(CommentError::NoRemote.into())
    }

    fn edit_pending(&self, _id: &str, _body: &str) -> Result<()> {
        Err(CommentError::NoRemote.into())
    }

    /// With no host, no comment is there, and taking back one that is not
    /// there is not an error. Refusing left a thread from before the remote
    /// went holding a pending comment it could never let go of.
    fn remove_pending(&self, _id: &str) -> Result<()> {
        Ok(())
    }

    fn remote(&self, ids: &[String]) -> Result<Vec<RemoteComment>> {
        Ok(ids
            .iter()
            .map(|id| RemoteComment {
                id: id.clone(),
                state: crate::comments::domain::RemoteState::Gone,
            })
            .collect())
    }

    fn pending_comments(
        &self,
        _pull: &str,
    ) -> Result<Vec<crate::comments::domain::PendingComment>> {
        Err(CommentError::NoRemote.into())
    }

    fn submit(&self, _review: &Review) -> Result<String> {
        Err(CommentError::NoRemote.into())
    }

    fn mark_read(&self, _pull: &str, _paths: &[String]) -> Result<usize> {
        Err(CommentError::NoRemote.into())
    }

    fn mark_unread(&self, _pull: &str, _paths: &[String]) -> Result<usize> {
        Err(CommentError::NoRemote.into())
    }

    fn viewed(&self, _pull: &str) -> Result<Vec<String>> {
        Err(CommentError::NoRemote.into())
    }

    fn open_pull(
        &self,
        _base: &str,
        _branch: &str,
        _draft: &crate::comments::domain::PullDraft,
    ) -> Result<crate::comments::domain::OpenedPull> {
        Err(CommentError::NoRemote.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comments::domain::RemoteState;

    #[test]
    fn with_no_host_every_comment_is_gone_and_letting_one_go_succeeds() {
        // A thread written before the remote went still names a pending
        // comment. It has to be able to let go of it.
        let host = Unhosted;
        assert_eq!(
            host.remote(&["PRRC_1".into()]).unwrap()[0].state,
            RemoteState::Gone
        );
        assert!(host.remove_pending("PRRC_1").is_ok());
    }
}
