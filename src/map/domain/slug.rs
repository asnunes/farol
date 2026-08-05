use serde::{Deserialize, Serialize};

use crate::shared::error::{Error, Result};

/// A block's identity: short, kebab-case, chosen by whoever writes the map.
///
/// It is a type rather than a `String` for two reasons. The format is checked
/// once, at the edge, instead of being hoped for everywhere. And a use case
/// taking `(&Slug, &ReviewPath)` cannot have its two arguments swapped by
/// accident, which `(&str, &str)` invites.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Slug(String);

impl Slug {
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        let shaped = !trimmed.is_empty()
            && trimmed
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !trimmed.starts_with('-')
            && !trimmed.ends_with('-');

        if !shaped {
            return Err(Error::BadSlug {
                raw: raw.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Slug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for Slug {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_kebab_case_is_accepted() {
        assert_eq!(
            Slug::parse("recover-link").unwrap().as_str(),
            "recover-link"
        );
        assert_eq!(Slug::parse("outros").unwrap().as_str(), "outros");
        assert_eq!(Slug::parse("step-2").unwrap().as_str(), "step-2");
    }

    #[test]
    fn surrounding_whitespace_is_forgiven() {
        assert_eq!(Slug::parse("  core  ").unwrap().as_str(), "core");
    }

    #[test]
    fn shapes_that_would_read_badly_in_a_command_are_refused() {
        for bad in [
            "",
            "   ",
            "Recover-Link", // the agent would then have to remember the case
            "recover link", // a space breaks the command it goes into
            "recover_link",
            "-leading",
            "trailing-",
            "src/a.rs", // a path where a slug was meant
        ] {
            assert!(
                Slug::parse(bad).is_err(),
                "{bad:?} should have been refused"
            );
        }
    }
}
