//! What a lens asks, and about what.

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use super::LensError;

/// A question the session answers about the change: a title, the group it is
/// asked in, and the prompt the session follows. The prompt can be no more
/// than "Use the X skill"; farol hands it over and does not run anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LensDefinition {
    pub id: LensId,
    pub title: String,
    pub group: LensGroup,
    pub prompt: String,
}

impl LensDefinition {
    /// What an answer remembers of the prompt it followed, so a prompt edited
    /// afterwards marks the answer as written against something else.
    pub fn prompt_hash(&self) -> String {
        stable_hash(&self.prompt)
    }

    /// About how long an answer to this lens should run, in lines: a guide
    /// handed to the session, never a wall. Refusing past a count made the
    /// session compress until the reader had to rebuild the meaning, which
    /// costs more than the lines it saved.
    pub fn about_lines(&self) -> usize {
        super::built_in::built_in(self.id.as_str())
            .map(|lens| lens.about_lines)
            .unwrap_or(DEFAULT_ABOUT_LINES)
    }
}

const DEFAULT_ABOUT_LINES: usize = 6;

/// What a lens is written about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LensGroup {
    /// The pull request as a whole, once.
    Pr,
    /// Each block, once.
    Block,
    /// Each file the prompt applies to, as the session judges.
    File,
}

impl LensGroup {
    pub fn parse(raw: &str) -> Result<Self, LensError> {
        match raw {
            "pr" => Ok(Self::Pr),
            "block" => Ok(Self::Block),
            "file" => Ok(Self::File),
            other => Err(LensError::UnknownGroup(other.to_string())),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Pr => "pr",
            Self::Block => "block",
            Self::File => "file",
        }
    }
}

/// What the session types: `farol lens write tests --file …`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LensId(String);

impl LensId {
    pub fn parse(raw: &str) -> Result<Self, LensError> {
        let ok = !raw.is_empty()
            && !raw.starts_with('-')
            && !raw.ends_with('-')
            && raw
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if ok {
            Ok(Self(raw.to_string()))
        } else {
            Err(LensError::BadId(raw.to_string()))
        }
    }

    /// `Decisões de UX` → `decisoes-de-ux`: something a session can type,
    /// folding the accents a Portuguese title carries rather than dropping the
    /// letters under them.
    pub fn from_title(title: &str) -> Self {
        // NFD splits `õ` into `o` and its tilde; dropping the marks keeps the
        // letter. Already in the build through gix, so it costs no new crate.
        let folded: String = title
            .to_lowercase()
            .nfd()
            .filter(|c| !is_combining_mark(*c))
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let id = folded
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        Self(if id.is_empty() { "lens".into() } else { id })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LensId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for LensId {
    type Error = LensError;
    fn try_from(raw: String) -> Result<Self, LensError> {
        Self::parse(&raw)
    }
}

impl From<LensId> for String {
    fn from(id: LensId) -> String {
        id.0
    }
}

/// FNV-1a, written out: the name and the prompt mark are kept on disk, and a
/// hasher whose output may change between Rust releases would turn every
/// stored answer into a stranger after an upgrade.
pub fn stable_hash(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_made_from_a_title_is_something_a_session_can_type() {
        assert_eq!(
            LensId::from_title("Decisões de UX").as_str(),
            "decisoes-de-ux"
        );
        assert_eq!(LensId::from_title("  ").as_str(), "lens");
        assert!(LensId::parse("Tests").is_err());
        assert!(LensId::parse("tests").is_ok());
    }

    #[test]
    fn the_stable_hash_does_not_move() {
        // Stored on disk: a change here turns every stored answer Outdated.
        assert_eq!(stable_hash(""), "cbf29ce484222325");
        assert_eq!(stable_hash("a"), "af63dc4c8601ec8c");
    }
}
