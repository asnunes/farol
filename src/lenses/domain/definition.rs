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

    /// Refuse an answer longer than the lens allows. A lens exists to spare
    /// the reader the code; asking for brevity in the prompt was not enough,
    /// so the length is a wall the session hits, not a wish it can ignore.
    pub fn hold_to_length(&self, text: &str) -> Result<(), LensError> {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let most = self.max_lines();
        if lines.len() > most {
            return Err(LensError::TooLong {
                id: self.id.to_string(),
                lines: lines.len(),
                most,
            });
        }
        if let Some(line) = lines.iter().find(|l| l.chars().count() > MAX_LINE) {
            return Err(LensError::LineTooLong {
                id: self.id.to_string(),
                line: line.trim().chars().take(60).collect(),
                most: MAX_LINE,
            });
        }
        Ok(())
    }

    /// How many lines the lens may take: its own limit when farol ships it,
    /// and a short default for one the user made.
    pub fn max_lines(&self) -> usize {
        super::built_in::built_in(self.id.as_str())
            .map(|lens| lens.max_lines)
            .unwrap_or(DEFAULT_MAX_LINES)
    }
}

/// The longest line a lens may have, in characters: one line on a laptop
/// screen, citation included.
pub const MAX_LINE: usize = 140;
const DEFAULT_MAX_LINES: usize = 6;

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

    #[test]
    fn an_answer_past_its_lens_limit_is_refused_and_says_so() {
        let summary = LensDefinition {
            id: LensId::parse("smell").unwrap(),
            title: "Smell".into(),
            group: LensGroup::File,
            prompt: "p".into(),
        };

        assert!(summary.hold_to_length("- one\n- two\n- three").is_ok());
        let long = summary
            .hold_to_length("- one\n- two\n- three\n- four")
            .unwrap_err()
            .to_string();
        assert!(long.contains("4 lines and may have at most 3"), "{long}");
        let wide = summary
            .hold_to_length(&"x".repeat(MAX_LINE + 1))
            .unwrap_err()
            .to_string();
        assert!(
            wide.contains("One line, one point"),
            "a line too wide is refused too: {wide}"
        );
    }
}
