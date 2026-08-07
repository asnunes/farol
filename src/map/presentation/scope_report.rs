//! The files under review, as farol resolved them.

use std::fmt::{self, Display};

use crate::diff::domain::Scope;

/// The files under review, as farol resolved them.
pub struct ScopeReport<'a>(pub &'a Scope);

impl Display for ScopeReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope = self.0;
        let separator = if scope.merge_base { "..." } else { ".." };
        writeln!(
            f,
            "review scope for {}{separator}{}",
            scope.base_ref, scope.head_ref
        )?;
        if scope.dirty {
            writeln!(f, "including uncommitted changes")?;
        }
        writeln!(f, "{} files\n", scope.files.len())?;

        for file in &scope.files {
            let rename = file
                .old_path
                .as_ref()
                .map(|p| format!(" (was {p})"))
                .unwrap_or_default();
            writeln!(
                f,
                "  {:<10} +{:<5} -{:<5} {}{rename}",
                file.status.label(),
                file.additions,
                file.deletions,
                file.path,
            )?;
        }
        Ok(())
    }
}
