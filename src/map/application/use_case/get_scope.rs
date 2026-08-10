use crate::diff::application::ReviewScope;
use crate::diff::domain::{ReviewPath, Scope};
use crate::error::Result;

/// The files under review, as farol resolved them.
#[derive(Clone)]
pub struct GetScope {
    scope: ReviewScope,
}

impl GetScope {
    pub fn new(scope: ReviewScope) -> Self {
        Self { scope }
    }

    pub fn execute(&self) -> Result<Scope> {
        self.scope.get().cloned()
    }

    /// Turning a raw path into a proven one is part of reading the scope, so it
    /// lives with it rather than in every use case that needs a path.
    pub fn path(&self, raw: &str) -> Result<ReviewPath> {
        self.scope.path(raw)
    }

    pub fn paths(&self, raw: &[String]) -> Result<Vec<ReviewPath>> {
        self.scope.paths(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeDiffSource;
    use std::sync::Arc;

    fn on(paths: &[&str]) -> GetScope {
        GetScope::new(ReviewScope::new(Arc::new(FakeDiffSource::with_paths(
            paths,
        ))))
    }

    #[test]
    fn the_scope_lists_what_the_branch_changed() {
        let scope = on(&["src/a.rs", "src/b.rs"]).execute().unwrap();

        let paths: Vec<&str> = scope.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["src/a.rs", "src/b.rs"]);
        assert_eq!(scope.branch, "feature/x");
        assert_eq!(scope.base_ref, "main");
    }

    #[test]
    fn a_path_under_review_comes_back_proven() {
        let path = on(&["src/a.rs"]).path("src/a.rs").unwrap();

        assert_eq!(path.as_str(), "src/a.rs");
    }

    #[test]
    fn a_path_outside_the_review_is_refused_before_any_command_runs() {
        let err = on(&["src/a.rs"]).path("elsewhere.rs").unwrap_err();

        assert!(err.to_string().contains("elsewhere.rs"), "{err}");
    }

    #[test]
    fn resolving_several_keeps_their_order_and_fails_on_the_first_bad_one() {
        let scope = on(&["a.rs", "b.rs"]);

        let good = scope.paths(&["b.rs".into(), "a.rs".into()]).unwrap();
        let names: Vec<&str> = good.iter().map(|p| p.as_str()).collect();
        assert_eq!(names, vec!["b.rs", "a.rs"]);

        let err = scope.paths(&["a.rs".into(), "nope.rs".into()]).unwrap_err();
        assert!(err.to_string().contains("nope.rs"), "{err}");
    }
}
