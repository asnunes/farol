//! What a lens says about one target, as the session wrote it.

use serde::{Deserialize, Serialize};

use super::{Citation, LensId, stable_hash};
use crate::map::domain::Slug;

/// One lens's answer for one target.
///
/// It remembers what it was written against — the files it covers, by git's
/// id for their content, and the prompt it followed — so the screen can say
/// when either has moved since. It never claims to be current: a file that
/// did not change does not prove the answer still holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensAnswer {
    pub lens: LensId,
    pub target: LensTarget,
    /// Markdown. Empty is an answer too: the session looked and the lens does
    /// not apply, and saying so keeps it from being asked again.
    pub text: String,
    /// The commit it was written at, or `working` for uncommitted work.
    pub written_at: String,
    pub covers: Vec<Covered>,
    pub prompt_hash: String,
    /// Files the answer asks a person to read themselves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Citation>,
}

impl LensAnswer {
    /// Whether it was written against what is under review now.
    pub fn freshness(&self, now: &[Covered], prompt_hash: &str) -> Freshness {
        let changed = self
            .covers
            .iter()
            .filter(|was| !now.contains(was))
            .chain(now.iter().filter(|is| !self.covers.contains(is)))
            .map(|c| c.path.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if changed == 0 && self.prompt_hash == prompt_hash {
            Freshness::Written
        } else {
            Freshness::Outdated { changed }
        }
    }

    /// What a seen mark is pinned to: the words and when they were written,
    /// so a rewrite or a keep after a commit asks to be read again.
    pub fn fingerprint(&self) -> String {
        stable_hash(&format!("{}\0{}", self.written_at, self.text))
    }

    /// Where a seen mark is kept for this answer.
    pub fn seen_key(&self) -> String {
        format!("{}/{}", self.lens, self.target.file_name())
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// What a lens answer is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LensTarget {
    Pr,
    Block { slug: Slug },
    File { path: String },
}

impl LensTarget {
    /// The answer's file name under the lens's folder. A path becomes a fixed
    /// length key: the real target is kept inside, so nothing is ever read back
    /// from the name, and a path deep enough to pass the file system's limit
    /// on a name, or one that is a prefix of another's folder, cannot break it.
    pub fn file_name(&self) -> String {
        match self {
            Self::Pr => "pr.json".into(),
            Self::Block { slug } => format!("block/{slug}.json"),
            Self::File { path } => format!("file/{}.json", stable_hash(path)),
        }
    }
}

impl std::fmt::Display for LensTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pr => f.write_str("the pull request"),
            Self::Block { slug } => write!(f, "block '{slug}'"),
            Self::File { path } => write!(f, "{path}"),
        }
    }
}

/// A file as the answer saw it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Covered {
    pub path: String,
    pub blob: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Written,
    /// The files it covers or its prompt changed since. `changed` counts the
    /// files, and is zero when only the prompt did.
    Outdated {
        changed: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covered(path: &str, blob: &str) -> Covered {
        Covered {
            path: path.into(),
            blob: blob.into(),
        }
    }

    fn answer(covers: Vec<Covered>) -> LensAnswer {
        LensAnswer {
            lens: LensId::parse("tests").unwrap(),
            target: LensTarget::Pr,
            text: "x".into(),
            written_at: "abc".into(),
            covers,
            prompt_hash: "p".into(),
            read: vec![],
            citations: vec![],
        }
    }

    #[test]
    fn an_answer_is_outdated_when_a_file_it_covers_entered_left_or_changed() {
        let a = answer(vec![covered("a.rs", "1"), covered("b.rs", "1")]);

        assert_eq!(
            a.freshness(&[covered("a.rs", "1"), covered("b.rs", "1")], "p"),
            Freshness::Written
        );
        assert_eq!(
            a.freshness(&[covered("a.rs", "2"), covered("b.rs", "1")], "p"),
            Freshness::Outdated { changed: 1 }
        );
        assert_eq!(
            a.freshness(&[covered("a.rs", "1")], "p"),
            Freshness::Outdated { changed: 1 }
        );
        assert_eq!(
            a.freshness(
                &[
                    covered("a.rs", "1"),
                    covered("b.rs", "1"),
                    covered("c.rs", "1")
                ],
                "p"
            ),
            Freshness::Outdated { changed: 1 }
        );
    }

    #[test]
    fn an_answer_is_outdated_when_its_prompt_changed() {
        let a = answer(vec![covered("a.rs", "1")]);
        assert_eq!(
            a.freshness(&[covered("a.rs", "1")], "q"),
            Freshness::Outdated { changed: 0 }
        );
    }

    #[test]
    fn a_file_target_is_stored_under_a_name_of_fixed_length() {
        let deep = LensTarget::File {
            path: "a/".repeat(200) + "b.rs",
        };
        let name = deep.file_name();
        assert!(name.len() < 40, "{name}");
        assert_ne!(
            LensTarget::File {
                path: "src/a-b.rs".into()
            }
            .file_name(),
            LensTarget::File {
                path: "src-a/b.rs".into()
            }
            .file_name()
        );
    }
}
