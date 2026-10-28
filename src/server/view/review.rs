//! The review as the screen draws it: blocks in the author's order, each file
//! under the one block that reads it, with that block's notes.

use std::collections::BTreeSet;

use serde::Serialize;

use super::LensView;
use crate::diff::domain::FileChange;
use crate::lenses::application::LensReading;
use crate::map::application::ReviewSnapshot;
use crate::map::domain::ReviewMap;
use crate::progress::domain::Progress;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewView {
    pub branch: String,
    pub base: String,
    pub generated_at: String,
    pub commits_behind: u32,
    pub blocks: Vec<BlockView>,
    /// Skim entries that belong to no block.
    pub loose_skim: Vec<FileView>,
    /// Files in the diff that no block mentions — only possible when the map is
    /// behind the branch. Shown as a warning, never silently dropped.
    pub unmapped: Vec<String>,
    pub total_files: usize,
    pub viewed_files: usize,
    /// The first file in reading order nobody has read yet, and nothing once
    /// they all have been.
    ///
    /// Answered here because it is a fact about the review — the order the
    /// author arranged and what the reader has got through — and both of those
    /// are already known on this side. Worked out in the browser it was worked
    /// out twice, once to decide where to put the reader when the map arrives
    /// and once to wrap the key that walks to the next unread file, each with
    /// its own idea of what to do when there is none left.
    pub first_unread: Option<String>,
    /// The pull request's lenses, in order, before the first block.
    pub lenses: Vec<LensView>,
}

impl ReviewView {
    /// Shape a snapshot for the wire. The use case gathered the parts; this
    /// only decides how the screen sees them.
    pub fn build(snapshot: &ReviewSnapshot, lenses: &LensReading) -> Self {
        let map = &snapshot.map;
        let scope = &snapshot.scope;
        let progress = &snapshot.progress;
        let commits_behind = snapshot.commits_behind;

        let mut blocks = Vec::new();
        let mut placed: Vec<String> = Vec::new();

        for block in map.blocks() {
            // A block whose every file left the comparison is gone from the
            // screen with them: a band of prose over nothing reads as a bug.
            if !map.paths_in(&block.slug).any(|path| scope.contains(path)) {
                continue;
            }

            let mut files = Vec::new();

            for bf in &block.files {
                let Some(change) = scope.change(&bf.path) else {
                    continue;
                };
                placed.push(bf.path.clone());
                files.push(FileView::build(map, change, progress, lenses, false, None));
            }

            // A file the block reads and also marks skim is drawn once, as
            // the file.
            for entry in map.skim_for(&block.slug) {
                if placed.contains(&entry.path) {
                    continue;
                }
                let Some(change) = scope.change(&entry.path) else {
                    continue;
                };
                placed.push(entry.path.clone());
                files.push(FileView::build(
                    map,
                    change,
                    progress,
                    lenses,
                    true,
                    Some(entry.reason.clone()),
                ));
            }

            // Over everything the block accounts for, read files and skim
            // alike, and each path once however many roles it plays in here.
            let held: BTreeSet<&str> = map
                .paths_in(&block.slug)
                .filter(|path| scope.contains(path))
                .collect();
            let viewed_files = held
                .iter()
                .filter_map(|path| scope.change(path))
                // The same question `FileView` asks of a row: read, and still
                // the same file.
                .filter(|change| progress.is_current(&change.path, &change.content_hash))
                .count();

            blocks.push(BlockView {
                slug: block.slug.to_string(),
                title: block.title.clone(),
                context: block.context.clone(),
                lenses: LensView::all(lenses.for_block(&block.slug), progress),
                files,
                total_files: held.len(),
                viewed_files,
            });
        }

        let mut loose_skim = Vec::new();
        for entry in map.loose_skim() {
            let Some(change) = scope.change(&entry.path) else {
                continue;
            };
            placed.push(entry.path.clone());
            loose_skim.push(FileView::build(
                map,
                change,
                progress,
                lenses,
                true,
                Some(entry.reason.clone()),
            ));
        }

        let unmapped: Vec<String> = scope
            .files
            .iter()
            .map(|f| f.path.clone())
            .filter(|p| !placed.contains(p))
            .collect();

        // The order the reader meets the files in: the blocks as the author
        // arranged them, then the skim that belongs to no block. Written once
        // and walked twice, because the count and where to resume are two
        // questions about the same walk.
        let in_reading_order = || {
            blocks
                .iter()
                .flat_map(|b| b.files.iter())
                .chain(loose_skim.iter())
        };

        let total_files = placed.len();
        let viewed_files = in_reading_order().filter(|f| f.viewed).count();
        let first_unread = in_reading_order()
            .find(|f| !f.viewed)
            .map(|f| f.path.clone());

        ReviewView {
            branch: scope.branch.clone(),
            base: scope.base_ref.clone(),
            generated_at: map.generated_at.clone(),
            commits_behind,
            blocks,
            loose_skim,
            unmapped,
            total_files,
            viewed_files,
            first_unread,
            lenses: LensView::all(lenses.for_pr(), progress),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockView {
    pub slug: String,
    pub title: String,
    pub context: String,
    /// The block's lenses, in order, between its context and its code.
    pub lenses: Vec<LensView>,
    pub files: Vec<FileView>,
    /// What the block holds, read files and skim alike, each path once.
    ///
    /// Never zero: a block with no path left in the comparison is not sent.
    pub total_files: usize,
    pub viewed_files: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub path: String,
    pub status: &'static str,
    pub additions: u32,
    pub deletions: u32,
    pub viewed: bool,
    /// The note the block wrote for this file.
    pub note: Option<String>,
    pub line_notes: Vec<LineNoteView>,
    pub skim: bool,
    pub skim_reason: Option<String>,
    /// The file's own lenses, drawn under its header.
    pub lenses: Vec<LensView>,
    /// Whether its diff starts folded: on a review the lenses speak for, code
    /// is opened by the reader, never shown to them by default.
    pub folded: bool,
    /// The lens that asks a person to read this file themselves.
    pub read_yourself: Option<String>,
}

impl FileView {
    /// Only ever called with the change git reports for the path. A file the
    /// comparison no longer holds has no status and no line counts to show, and
    /// inventing them is what left merged branches offering diffs the server
    /// then refused.
    fn build(
        map: &ReviewMap,
        change: &FileChange,
        progress: &Progress,
        lenses: &LensReading,
        skim: bool,
        skim_reason: Option<String>,
    ) -> FileView {
        let path = change.path.as_str();

        let file = map.block_of(path).and_then(|block| block.file(path));
        let note = file.and_then(|f| f.note.clone());
        let mut line_notes: Vec<LineNoteView> = file
            .map(|f| {
                f.line_notes
                    .iter()
                    .map(|n| LineNoteView {
                        from: n.range.from,
                        to: n.range.to,
                        text: n.text.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        line_notes.sort_by_key(|n| (n.from, n.to));

        // Read *and still the same file*. Comparing paths alone was what kept
        // a file struck through after it had changed under the mark, which is
        // the one thing this is supposed to catch.
        let viewed = progress.is_current(path, &change.content_hash);

        FileView {
            path: path.to_string(),
            status: change.status.label(),
            additions: change.additions,
            deletions: change.deletions,
            viewed,
            note,
            line_notes,
            skim,
            skim_reason,
            lenses: LensView::all(lenses.for_file(path), progress),
            folded: lenses.speaks(),
            read_yourself: lenses.read_yourself(path).map(str::to_string),
        }
    }
}

/// A note pinned to a span of lines on the new side.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineNoteView {
    pub from: u32,
    pub to: u32,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::domain::ReviewScopeSource;
    use crate::map::domain::{LineRange, Position};
    use crate::testing::{FakeDiffSource, slug};

    /// Assemble the snapshot the way the use case would, from a fake.
    fn built(map: &ReviewMap, source: FakeDiffSource, progress: &Progress) -> ReviewView {
        let scope = source.scope().unwrap().clone();
        ReviewView::build(
            &ReviewSnapshot {
                map: map.clone(),
                scope,
                progress: progress.clone(),
                commits_behind: 0,
            },
            &crate::lenses::application::LensReading::default(),
        )
    }

    fn map_of(paths: &[(&str, &str)]) -> ReviewMap {
        let mut m = ReviewMap::new("feature/x", "main", "head");
        for (name, _) in paths {
            if m.block(&slug(name)).is_none() {
                m.add_block(&slug(name), "t", "c", Position::End).unwrap();
            }
        }
        for (name, path) in paths {
            m.add_file(&slug(name), *path, None, None).unwrap();
        }
        m
    }

    #[test]
    fn skim_with_a_block_sits_inside_it_and_loose_skim_stays_at_the_bottom() {
        let mut map = map_of(&[("one", "a.rs")]);
        map.add_skim("a_test.rs", "fixture only", Some(slug("one")))
            .unwrap();
        map.add_skim("Cargo.lock", "generated", None).unwrap();

        let source = FakeDiffSource::with_paths(&["a.rs", "a_test.rs", "Cargo.lock"]);
        let view = built(&map, source, &Progress::new());

        assert_eq!(view.blocks[0].files.len(), 2);
        assert!(view.blocks[0].files[1].skim);
        assert_eq!(view.loose_skim.len(), 1);
        assert_eq!(view.loose_skim[0].path, "Cargo.lock");
    }

    #[test]
    fn a_mapped_file_the_comparison_no_longer_holds_is_not_offered_at_all() {
        // The base moved under the map — a merge, typically. The map still
        // names the file, git no longer does, and offering it anyway left the
        // pane asking for a diff the server refuses and loading forever.
        let mut map = map_of(&[("one", "a.rs"), ("one", "gone.rs")]);
        map.add_skim("skimmed_away.rs", "generated", Some(slug("one")))
            .unwrap();
        map.add_skim("loose_gone.rs", "generated", None).unwrap();

        let view = built(
            &map,
            FakeDiffSource::with_paths(&["a.rs"]),
            &Progress::new(),
        );

        let paths: Vec<_> = view.blocks[0].files.iter().map(|f| &f.path).collect();
        assert_eq!(paths, vec!["a.rs"]);
        assert!(view.loose_skim.is_empty());
        assert_eq!(view.total_files, 1);
        assert!(view.unmapped.is_empty());
    }

    #[test]
    fn an_empty_comparison_leaves_nothing_on_the_screen_to_read() {
        // Every block went with its files: a band of prose over no diff reads
        // as a bug. The map on disk is untouched — this is the view saying
        // there is nothing here, not the map being repaired.
        let map = map_of(&[("one", "a.rs"), ("two", "b.rs")]);

        let view = built(&map, FakeDiffSource::with_paths(&[]), &Progress::new());

        assert!(view.blocks.is_empty(), "{:?}", view.blocks);
        assert!(view.loose_skim.is_empty());
        assert_eq!((view.total_files, view.viewed_files), (0, 0));
    }

    #[test]
    fn where_to_resume_is_the_first_file_in_reading_order_nobody_has_read() {
        // The order the author arranged, not the order git lists paths in.
        let map = map_of(&[("one", "b.rs"), ("one", "a.rs")]);
        let mut progress = Progress::new();
        progress.mark("b.rs", "hash-of-b.rs", "now");

        let view = built(
            &map,
            FakeDiffSource::with_paths(&["a.rs", "b.rs"]),
            &progress,
        );

        assert_eq!(view.first_unread.as_deref(), Some("a.rs"));
    }

    #[test]
    fn a_review_read_to_the_end_has_nowhere_left_to_resume() {
        // Not the first file as a fallback: the key that walks to the next
        // unread file would then jump to the top of a finished review instead
        // of leaving the reader where they are.
        let map = map_of(&[("one", "a.rs")]);
        let mut progress = Progress::new();
        progress.mark("a.rs", "hash-of-a.rs", "now");

        let view = built(&map, FakeDiffSource::with_paths(&["a.rs"]), &progress);

        assert_eq!(view.first_unread, None);
    }

    #[test]
    fn loose_skim_is_reached_last_and_can_be_where_the_reader_resumes() {
        // It sits at the bottom of the page, so it is the last thing left to
        // read — and the only thing left once the blocks are done.
        let mut map = map_of(&[("one", "a.rs")]);
        map.add_skim("Cargo.lock", "generated", None).unwrap();
        let mut progress = Progress::new();
        progress.mark("a.rs", "hash-of-a.rs", "now");

        let view = built(
            &map,
            FakeDiffSource::with_paths(&["a.rs", "Cargo.lock"]),
            &progress,
        );

        assert_eq!(view.first_unread.as_deref(), Some("Cargo.lock"));
    }

    #[test]
    fn skim_attached_to_a_block_counts_towards_finishing_it() {
        // A skimmed file is still a file the reader ticks off, and the progress
        // over the whole review counts it. A block that left it out would sit
        // at done while a row under it was unticked.
        let mut map = map_of(&[("one", "a.rs")]);
        map.add_skim("gen.lock", "generated", Some(slug("one")))
            .unwrap();

        let view = built(
            &map,
            FakeDiffSource::with_paths(&["a.rs", "gen.lock"]),
            &Progress::new(),
        );

        assert_eq!(
            (view.blocks[0].total_files, view.blocks[0].viewed_files),
            (2, 0)
        );
    }

    #[test]
    fn a_file_read_before_it_left_the_comparison_stops_counting_towards_progress() {
        // Otherwise the bar reads 2 / 1 once half the scope goes away.
        let map = map_of(&[("one", "a.rs"), ("one", "gone.rs")]);
        let mut progress = Progress::new();
        progress.mark("a.rs", "hash-of-a.rs", "now");
        progress.mark("gone.rs", "hash-of-gone.rs", "now");

        let view = built(&map, FakeDiffSource::with_paths(&["a.rs"]), &progress);

        assert_eq!((view.total_files, view.viewed_files), (1, 1));
    }

    #[test]
    fn files_the_map_never_mentions_are_reported_not_hidden() {
        let map = map_of(&[("one", "a.rs")]);
        let source = FakeDiffSource::with_paths(&["a.rs", "arrived_later.rs"]);
        let view = built(&map, source, &Progress::new());
        assert_eq!(view.unmapped, vec!["arrived_later.rs"]);
    }

    #[test]
    fn viewed_count_reflects_progress() {
        let map = map_of(&[("one", "a.rs"), ("one", "b.rs")]);
        let source = FakeDiffSource::with_paths(&["a.rs", "b.rs"]);
        let mut progress = Progress::new();
        progress.mark("a.rs", "hash-of-a.rs", "now");

        let view = built(&map, source, &progress);
        assert_eq!(view.total_files, 2);
        assert_eq!(view.viewed_files, 1);
    }

    #[test]
    fn a_file_that_changed_under_the_mark_comes_back_unread() {
        // The whole point of anchoring progress to the content: a rebase that
        // only moves the file leaves the mark alone, and a real change takes it
        // off. Comparing paths alone left every file struck through for good,
        // and this test passed anyway because it marked a hash that matched
        // nothing.
        let map = map_of(&[("one", "a.rs")]);
        let source = FakeDiffSource::with_paths(&["a.rs"]);
        let mut progress = Progress::new();
        progress.mark("a.rs", "the-hash-it-had-before-the-fix", "now");

        let view = built(&map, source, &progress);

        assert_eq!(view.viewed_files, 0);
        assert!(!view.blocks[0].files[0].viewed);
    }

    #[test]
    fn a_file_marked_skim_inside_a_block_it_already_appears_in_is_not_listed_twice() {
        // The reviewer would otherwise meet the same path twice in one block,
        // once to read and once to skim.
        let mut map = map_of(&[("first", "a.rs")]);
        map.add_skim("a.rs", "mostly generated", Some(slug("first")))
            .unwrap();

        let view = built(
            &map,
            FakeDiffSource::with_paths(&["a.rs"]),
            &Progress::new(),
        );

        let first = &view.blocks[0];
        assert_eq!(
            first.files.iter().filter(|f| f.path == "a.rs").count(),
            1,
            "{:?}",
            first.files.iter().map(|f| &f.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_loose_skim_entry_already_read_in_a_block_does_not_reappear_at_the_bottom() {
        let mut map = map_of(&[("first", "a.rs")]);
        map.add_skim("a.rs", "mostly generated", None).unwrap();

        let view = built(
            &map,
            FakeDiffSource::with_paths(&["a.rs"]),
            &Progress::new(),
        );

        assert!(
            !view.loose_skim.iter().any(|f| f.path == "a.rs"),
            "it was already placed inside the block: {:?}",
            view.loose_skim.iter().map(|f| &f.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn line_notes_arrive_in_the_order_the_reader_meets_them() {
        // They are written in whatever order the session thought of them; the
        // screen shows them going down the file.
        let mut map = map_of(&[("first", "a.rs")]);
        for (from, to, text) in [(40, 42, "later"), (10, 12, "earlier")] {
            map.add_line_note(
                &slug("first"),
                "a.rs",
                LineRange::new(from, to).unwrap(),
                text,
            )
            .unwrap();
        }

        let view = built(
            &map,
            FakeDiffSource::with_paths(&["a.rs"]),
            &Progress::new(),
        );

        let file = view.blocks[0]
            .files
            .iter()
            .find(|f| f.path == "a.rs")
            .unwrap();
        let spans: Vec<_> = file.line_notes.iter().map(|n| (n.from, n.to)).collect();
        assert_eq!(spans, vec![(10, 12), (40, 42)]);
    }

    #[test]
    fn with_lenses_written_every_file_starts_folded_and_carries_its_own_lens() {
        use crate::lenses::application::LensReading;
        use crate::lenses::domain::{
            Covered, Freshness, LensAnswer, LensCatalog, LensId, LensTarget,
        };
        let map = map_of(&[("core", "a.rs"), ("core", "b.rs")]);
        let source = FakeDiffSource::with_paths(&["a.rs", "b.rs"]);
        let scope = source.scope().unwrap().clone();
        let blob = |p: &str| scope.change(p).unwrap().content_hash.clone();
        let tests = LensAnswer {
            lens: LensId::parse("tests").unwrap(),
            target: LensTarget::File {
                path: "a.rs".into(),
            },
            text: "- retries `a.rs:3`".into(),
            written_at: "abc1234".into(),
            covers: vec![Covered {
                path: "a.rs".into(),
                blob: blob("a.rs"),
            }],
            prompt_hash: "p".into(),
            read: vec!["b.rs".into()],
            citations: vec![],
        };
        let reading = LensReading {
            lenses: LensCatalog::default().enabled(),
            answers: vec![(tests, Freshness::Written)],
        };

        let view = ReviewView::build(
            &ReviewSnapshot {
                map,
                scope,
                progress: Progress::new(),
                commits_behind: 0,
            },
            &reading,
        );

        let files = &view.blocks[0].files;
        assert!(
            files.iter().all(|f| f.folded),
            "once lenses are written no file is shown until it is opened"
        );
        assert_eq!(files[0].lenses[0].title, "Tests");
        assert_eq!(files[1].read_yourself.as_deref(), Some("Tests"));
        let pr: Vec<_> = view
            .lenses
            .iter()
            .map(|l| (l.id.as_str(), l.state))
            .collect();
        assert_eq!(
            pr,
            vec![
                ("overview", "missing"),
                ("risk", "missing"),
                ("evidence", "missing")
            ],
            "the pull request's lenses are drawn, written or not"
        );
    }
}
