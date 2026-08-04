//! Every write to the map goes through here, so validation happens at the point
//! of the call.
//!
//! That is the whole reason the skill talks to a CLI instead of authoring a
//! document: a hallucinated path or an impossible range is rejected immediately,
//! with a message the agent can act on, instead of landing in a file and
//! surfacing as a blank spot on the screen later.

use crate::diff::domain::DiffSource;
use crate::map::domain::{MapRepository, Position, ReviewMap};
use crate::shared::error::{Error, Result};

pub struct MapWriter<'a> {
    pub source: &'a dyn DiffSource,
    pub repo: &'a dyn MapRepository,
}

impl<'a> MapWriter<'a> {
    /// Load the version for the current commit, apply `f`, store it back.
    pub fn edit<F>(&self, f: F) -> Result<ReviewMap>
    where
        F: FnOnce(&mut ReviewMap, &dyn DiffSource) -> Result<()>,
    {
        let derived = super::derive(self.source, self.repo)?;
        let mut map = derived.map;
        f(&mut map, self.source)?;
        self.repo.save(&map)?;
        Ok(map)
    }
}

/// Reject a path the reviewer will never be shown, and say what was probably
/// meant. Hallucinated paths are the most common way an agent gets this wrong.
pub fn require_in_scope(source: &dyn DiffSource, path: &str) -> Result<()> {
    let scope = source.scope()?;
    if scope.contains(path) {
        return Ok(());
    }
    Err(Error::PathOutOfScope {
        path: path.to_string(),
        similar: scope.similar_paths(path),
    })
}

/// A note pointing past the end of the file would render nowhere.
pub fn require_range_in_file(
    source: &dyn DiffSource,
    path: &str,
    from: u32,
    to: u32,
) -> Result<()> {
    if from == 0 || to < from {
        return Err(Error::BadRange {
            raw: format!("{from}-{to}"),
        });
    }
    let total = source.file_line_count(path)?;
    if to > total {
        return Err(Error::RangeOutOfFile {
            path: path.to_string(),
            from,
            to,
            total,
        });
    }
    Ok(())
}

pub fn parse_range(raw: &str) -> Result<(u32, u32)> {
    let (a, b) = raw.split_once('-').ok_or_else(|| Error::BadRange {
        raw: raw.to_string(),
    })?;
    let from = a.trim().parse::<u32>().map_err(|_| Error::BadRange {
        raw: raw.to_string(),
    })?;
    let to = b.trim().parse::<u32>().map_err(|_| Error::BadRange {
        raw: raw.to_string(),
    })?;
    if from == 0 || to < from {
        return Err(Error::BadRange {
            raw: raw.to_string(),
        });
    }
    Ok((from, to))
}

pub fn position_from(before: Option<String>, after: Option<String>) -> Position {
    match (before, after) {
        (Some(b), _) => Position::Before(b),
        (None, Some(a)) => Position::After(a),
        (None, None) => Position::End,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_parse_and_reject_nonsense() {
        assert_eq!(parse_range("82-116").unwrap(), (82, 116));
        assert_eq!(parse_range(" 5 - 9 ").unwrap(), (5, 9));
        assert!(parse_range("82").is_err());
        assert!(parse_range("0-9").is_err());
        assert!(parse_range("9-5").is_err());
        assert!(parse_range("a-b").is_err());
    }

    #[test]
    fn before_wins_over_after_when_both_are_given() {
        let p = position_from(Some("x".into()), Some("y".into()));
        assert_eq!(p, Position::Before("x".into()));
    }

    #[test]
    fn no_flags_means_append() {
        assert_eq!(position_from(None, None), Position::End);
    }
}
