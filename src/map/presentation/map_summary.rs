//! What the map holds, in one line.
//!
//! For the moment after deriving, when the reader needs a sense of what they
//! inherited without being handed the whole thing back. What is *missing* from
//! it is [`CheckSummary`](super::CheckSummary)'s job — this only counts what is
//! there.

use std::fmt::{self, Display};

use crate::map::domain::ReviewMap;

/// How many block names to print before giving up and counting them. Enough for
/// a map you can hold in your head; past that the names stop being a summary.
const NAMES_SHOWN: usize = 4;

pub struct MapSummary<'a>(pub &'a ReviewMap);

impl Display for MapSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let map = self.0;

        let mut parts = vec![match map.blocks().len() {
            1 => format!("1 block ({})", map.slugs().join(", ")),
            n if n <= NAMES_SHOWN => format!("{n} blocks ({})", map.slugs().join(", ")),
            n => format!("{n} blocks"),
        }];

        // A file in two blocks is one file to read, so it counts once.
        let files = map.covered_paths().len() - map.skim().len();
        parts.push(match files {
            1 => "1 file".to_string(),
            n => format!("{n} files"),
        });

        let notes: usize = map
            .blocks()
            .iter()
            .flat_map(|b| b.files.iter())
            .map(|file| file.line_notes.len())
            .sum();
        if notes > 0 {
            parts.push(match notes {
                1 => "1 line note".to_string(),
                n => format!("{n} line notes"),
            });
        }

        if !map.skim().is_empty() {
            parts.push(format!("{} marked skim", map.skim().len()));
        }

        writeln!(f, "{}", parts.join(" · "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::domain::Position;
    use crate::testing::{range, slug};

    fn mapped() -> ReviewMap {
        let mut m = ReviewMap::new("feature/x", "main", "abc123");
        m.add_block(&slug("core"), "t", "c", Position::End).unwrap();
        m.add_file(&slug("core"), "a.rs", None, None).unwrap();
        m
    }

    fn summary(map: &ReviewMap) -> String {
        MapSummary(map).to_string()
    }

    #[test]
    fn a_small_map_is_summarised_by_the_names_of_its_blocks() {
        // Two words say more about what is in there than any count.
        let mut m = mapped();
        m.add_block(&slug("wiring"), "t", "c", Position::End)
            .unwrap();
        m.add_file(&slug("wiring"), "b.rs", None, None).unwrap();

        assert!(
            summary(&m).starts_with("2 blocks (core, wiring)"),
            "{}",
            summary(&m)
        );
    }

    #[test]
    fn a_map_too_big_to_name_is_counted_instead() {
        // Past a handful the names stop summarising and start being the map.
        let mut m = ReviewMap::new("feature/x", "main", "abc123");
        for i in 0..8 {
            m.add_block(&slug(&format!("block-{i}")), "t", "c", Position::End)
                .unwrap();
        }

        let out = summary(&m);
        assert!(out.starts_with("8 blocks ·"), "{out}");
        assert!(!out.contains("block-0"), "{out}");
    }

    #[test]
    fn a_file_in_two_blocks_counts_once_because_it_is_read_once() {
        let mut m = mapped();
        m.add_block(&slug("wiring"), "t", "c", Position::End)
            .unwrap();
        m.add_file(&slug("wiring"), "a.rs", None, None).unwrap();

        assert!(
            summary(&m).contains("1 file ·") || summary(&m).contains("1 file\n"),
            "{}",
            summary(&m)
        );
    }

    #[test]
    fn line_notes_are_counted_because_they_are_the_expensive_part() {
        let mut m = mapped();
        m.add_line_note(&slug("core"), "a.rs", range(10, 12), "n")
            .unwrap();
        m.add_line_note(&slug("core"), "a.rs", range(40, 42), "n")
            .unwrap();

        assert!(summary(&m).contains("2 line notes"), "{}", summary(&m));
    }

    #[test]
    fn what_is_not_there_is_not_mentioned() {
        // A map with no notes should not report "0 line notes" — the reader is
        // being given a sense of the shape, not a form with empty fields.
        let out = summary(&mapped());

        assert_eq!(out.trim(), "1 block (core) · 1 file");
    }

    #[test]
    fn skim_entries_are_counted_apart_from_files_to_read() {
        let mut m = mapped();
        m.add_skim("Cargo.lock", "generated", None).unwrap();

        let out = summary(&m);
        assert!(out.contains("1 file ·"), "{out}");
        assert!(out.contains("1 marked skim"), "{out}");
    }
}
