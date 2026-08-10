use crate::error::Result;
use crate::map::application::{MapEditor, ResetOutcome};

/// Throw away the newest version so the one before it takes over.
#[derive(Clone)]
pub struct ResetMap {
    editor: MapEditor,
}

impl ResetMap {
    pub fn new(editor: MapEditor) -> Self {
        Self { editor }
    }

    pub fn execute(&self) -> Result<ResetOutcome> {
        self.editor.reset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::MapError;
    use crate::testing::{FakeDiffSource, InMemoryMapRepository, services};
    use std::sync::Arc;

    #[test]
    fn resetting_with_nothing_stored_here_says_so() {
        let svc = services(
            FakeDiffSource::with_paths(&["a.rs"]).on_commit("head"),
            Arc::new(InMemoryMapRepository::new()),
        );

        assert!(matches!(
            ResetMap::new(svc.editor).execute().unwrap(),
            ResetOutcome::NothingToDelete
        ));
    }

    #[test]
    fn resetting_falls_back_to_the_version_before_it() {
        let svc = services(
            FakeDiffSource::with_paths(&["a.rs"])
                .on_commit("head")
                .with_ancestors(&["old"])
                .at_distance("old", 1),
            Arc::new(InMemoryMapRepository::new()),
        );
        svc.editor.edit(|_| Ok::<_, MapError>(())).unwrap(); // a version for "head"

        let outcome = ResetMap::new(svc.editor).execute().unwrap();

        assert!(matches!(outcome, ResetOutcome::Deleted { .. }));
        assert!(
            svc.versions.current().unwrap().is_none(),
            "there was nothing before it to fall back to"
        );
    }
}
