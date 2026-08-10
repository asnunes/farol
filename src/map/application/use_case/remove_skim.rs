use crate::diff::domain::ReviewPath;
use crate::error::Result;
use crate::map::application::MapEditor;
use crate::map::domain::ReviewMap;

/// Take a file back off the skim list, when it turns out to deserve reading.
#[derive(Clone)]
pub struct RemoveSkim {
    maps: MapEditor,
}

impl RemoveSkim {
    pub fn new(maps: MapEditor) -> Self {
        Self { maps }
    }

    pub fn execute(&self, path: &ReviewPath) -> Result<ReviewMap> {
        self.maps.edit(|map| map.remove_skim(path.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::application::AddSkim;
    use crate::testing::use_case_setup;

    #[test]
    fn removing_one_that_is_not_marked_is_refused() {
        // Quietly doing nothing would let a typo look like it worked.
        let (svc, scope) = use_case_setup(&["Cargo.lock"]);

        assert!(
            RemoveSkim::new(svc.editor)
                .execute(&scope.path("Cargo.lock").unwrap())
                .is_err()
        );
    }

    #[test]
    fn removing_takes_the_entry_back_out() {
        let (svc, scope) = use_case_setup(&["Cargo.lock"]);
        let path = scope.path("Cargo.lock").unwrap();
        AddSkim::new(svc.editor.clone())
            .execute(&path, "generated", None)
            .unwrap();

        let map = RemoveSkim::new(svc.editor).execute(&path).unwrap();

        assert!(map.skim().is_empty());
    }
}
