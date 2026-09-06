//! The shape a map takes to leave the machine it was written on.
//!
//! Both directions read it, so it lives beside the other services rather than
//! inside either use case.

use crate::map::domain::{MapError, ReviewMap};

/// There is no version of its own: the map carries the only one there is, and a
/// second number beside it would be a second thing to keep in step.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Bundle {
    /// Two labels, neither compared. They are here to be read by whoever opens
    /// the file wondering where it came from.
    pub repo: String,
    pub branch: String,
    /// What identifies the review: two clones that share a base are the same
    /// review, whatever their branches are called.
    pub base: String,
    /// Where the map was written. Reported when it differs from here, never
    /// refused — reading a review a few commits ahead of its map is ordinary.
    pub head: String,
    pub map: ReviewMap,
}

impl Bundle {
    pub fn parse(raw: &str) -> Result<Self, MapError> {
        let bundle: Self =
            serde_json::from_str(raw).map_err(|e| MapError::NotAnExport { why: e.to_string() })?;
        // This value becomes a filename. Exported maps carry full SHA-1 IDs,
        // never revision expressions, paths, or the local working-tree marker.
        let commit = &bundle.map.generated_at;
        if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(MapError::NotAnExport {
                why: "map.generated_at must be a full 40-character hexadecimal commit ID\nExport the map again with `farol map export`.".into(),
            });
        }
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_version_keys_cannot_be_paths_or_revision_expressions() {
        for commit in [
            "",
            "working",
            "HEAD",
            "HEAD~1",
            "abc1234",
            "../outside",
            "/tmp/outside",
            r"..\outside",
            r"C:\outside",
            ".",
            "..",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/",
            "gggggggggggggggggggggggggggggggggggggggg",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            let error = Bundle::parse(&exported(commit))
                .err()
                .expect("an invalid storage key must be rejected");
            assert!(
                error.to_string().contains("map.generated_at"),
                "{commit:?}: {error}"
            );
        }
    }

    #[test]
    fn a_full_commit_id_survives_import_parsing() {
        for commit in [
            "0123456789abcdef0123456789abcdef01234567",
            "0123456789ABCDEF0123456789ABCDEF01234567",
        ] {
            let bundle = Bundle::parse(&exported(commit)).unwrap();
            assert_eq!(bundle.map.generated_at, commit);
        }
    }

    fn exported(commit: &str) -> String {
        serde_json::to_string(&Bundle {
            repo: "owner/repo".into(),
            branch: "feature/x".into(),
            base: "base".into(),
            head: commit.into(),
            map: ReviewMap::new("feature/x", "main", commit),
        })
        .unwrap()
    }
}
