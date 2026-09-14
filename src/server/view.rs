//! Turning the stored map into what the screen actually shows.
//!
//! The rendering rules live here rather than in the browser, so the frontend
//! stays a renderer: a file is read in the earliest block that holds it, its
//! notes from *every* block travel with it, and the tags say which stories it
//! belongs to.

use serde::Serialize;

use crate::comments::domain::{Comment, Found, Readiness};
use crate::comments::presentation::says;
use crate::diff::domain::{FileChange, FileDiff, Hunk, Line, LineKind};
use crate::map::application::ReviewSnapshot;
use crate::map::domain::ReviewMap;
use crate::progress::domain::Progress;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub path: String,
    pub status: &'static str,
    pub additions: u32,
    pub deletions: u32,
    pub viewed: bool,
    /// Note written for this file, from whichever block contributed it.
    pub notes: Vec<TaggedNote>,
    pub line_notes: Vec<TaggedLineNote>,
    /// Every block this file participates in, in reading order. More than one
    /// means the file shows up once but matters in several places.
    pub tags: Vec<String>,
    pub skim: bool,
    pub skim_reason: Option<String>,
}

/// One file's diff, as the pane reads it.
///
/// A view like every other answer rather than the domain's own `FileDiff`
/// serialised whole. Two reasons: the names arrive in the one convention the
/// browser uses, and what the domain grows next is not published by accident.
/// `old_path` and `new_content_hash` are on the stored type and stop here,
/// because the screen has never had a use for either.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffView {
    pub path: String,
    pub status: &'static str,
    pub hunks: Vec<HunkView>,
    /// git will not diff this file: binary content, or `-diff` in
    /// `.gitattributes`. There are no hunks, and the screen says so instead of
    /// rendering decoded bytes as if they were code.
    pub binary: bool,
    pub additions: u32,
    pub deletions: u32,
    /// How long the file is after the change, which is where the last gap the
    /// reader can open ends.
    pub line_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HunkView {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<LineView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineView {
    pub kind: &'static str,
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
    pub content: String,
}

impl FileDiffView {
    pub fn of(diff: &FileDiff) -> Self {
        Self {
            path: diff.path.clone(),
            status: diff.status.label(),
            hunks: diff.hunks.iter().map(HunkView::of).collect(),
            binary: diff.binary,
            additions: diff.additions,
            deletions: diff.deletions,
            line_count: diff.line_count,
        }
    }
}

impl HunkView {
    fn of(hunk: &Hunk) -> Self {
        Self {
            old_start: hunk.old_start,
            old_lines: hunk.old_lines,
            new_start: hunk.new_start,
            new_lines: hunk.new_lines,
            lines: hunk.lines.iter().map(LineView::of).collect(),
        }
    }
}

impl LineView {
    fn of(line: &Line) -> Self {
        Self {
            kind: match line.kind {
                LineKind::Context => "context",
                LineKind::Added => "added",
                LineKind::Removed => "removed",
            },
            old_number: line.old_number,
            new_number: line.new_number,
            content: line.content.clone(),
        }
    }
}

/// A stretch of the file the diff did not print, for a reader opening a gap.
///
/// The lines and nothing else: which line each one is, and which it was before
/// the change, the page works out from the hunks it already has.
#[derive(Debug, Serialize)]
pub struct LinesView {
    pub lines: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggedNote {
    pub block: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggedLineNote {
    pub block: String,
    pub from: u32,
    pub to: u32,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockView {
    pub slug: String,
    pub title: String,
    pub context: String,
    pub files: Vec<FileView>,
}

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
}

/// One comment, for the wire.
///
/// The domain keeps its own shape: comments are stored as markdown, so serde on
/// `Comment` would exist for nothing but this hop.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentView {
    pub id: String,
    pub path: String,
    pub from: u32,
    pub to: u32,
    pub body: String,
    /// Where it can be read on the pull request, once it has gone. The screen
    /// marks those rather than hiding them: publishing is not answering.
    pub published: Option<String>,
}

/// The comment list, and what the store could not read.
///
/// An object rather than a bare array because the unreadable files travel with
/// it: the page has to be able to say a comment went missing, and a list has
/// nowhere to put that.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentsView {
    pub comments: Vec<CommentView>,
    pub unreadable: Vec<UnreadableView>,
}

/// Whether the review can be sent, and when it cannot, what is in the way.
///
/// One object with a `state` the page switches on, because the difference
/// between the states is the whole content of the panel: each is a different
/// thing for the reader to go and do. `openAt` rides along on the one state
/// that has somewhere to send them.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessView {
    pub host: Option<String>,
    pub state: &'static str,
    /// The branch, in every state: the panel spells out commands with it in
    /// them, and it should not have to go and ask a second route for the name.
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<u32>,
    /// Whether the pull request is the reviewer's own, which decides what
    /// verdicts the screen can offer at all.
    pub mine: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_at: Option<String>,
}

impl ReadinessView {
    pub fn of(readiness: &Readiness, branch: &str, host: Option<&str>) -> Self {
        let mut view = Self {
            host: host.map(str::to_string),
            state: match readiness {
                Readiness::Ready { .. } => "ready",
                Readiness::NoRemote => "noRemote",
                Readiness::NoToken => "noToken",
                Readiness::TokenRefused => "tokenRefused",
                Readiness::BranchNotPushed => "branchNotPushed",
                Readiness::NoPullRequest { .. } => "noPullRequest",
            },
            branch: branch.to_string(),
            pull_request: None,
            mine: false,
            open_at: None,
        };
        match readiness {
            Readiness::Ready {
                pull_request, mine, ..
            } => {
                view.pull_request = Some(*pull_request);
                view.mine = *mine;
            }
            Readiness::NoPullRequest { open_at } => view.open_at = Some(open_at.clone()),
            _ => {}
        }
        view
    }
}

/// What a published review left behind: where it can be read, and how many
/// comments went with it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SentView {
    pub url: String,
    pub comments: usize,
    /// Files ticked as read on the pull request, and what stopped it when
    /// something did. The review went in both cases, which is why the screen
    /// says this beside the address rather than instead of it.
    pub read: usize,
    pub read_failed: Option<String>,
}

/// A comment the store could not read, in the terms the page speaks: the file
/// it was written about, or the start of the prose when the header no longer
/// says which file that was.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadableView {
    pub file: String,
    pub about: Option<String>,
    pub excerpt: Option<String>,
    pub why: &'static str,
}

impl ReviewView {
    /// Shape a snapshot for the wire. The use case gathered the parts; this
    /// only decides how the screen sees them.
    pub fn build(snapshot: &ReviewSnapshot) -> Self {
        let map = &snapshot.map;
        let scope = &snapshot.scope;
        let progress = &snapshot.progress;
        let commits_behind = snapshot.commits_behind;

        let mut blocks = Vec::new();
        let mut placed: Vec<String> = Vec::new();

        for block in map.blocks() {
            let mut files = Vec::new();

            for bf in &block.files {
                // Only the earliest block renders it; later ones would repeat the
                // same diff under a different heading.
                if placed.contains(&bf.path) {
                    continue;
                }
                let Some(change) = scope.change(&bf.path) else {
                    continue;
                };
                placed.push(bf.path.clone());
                files.push(FileView::build(map, change, progress, false, None));
            }

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
                    true,
                    Some(entry.reason.clone()),
                ));
            }

            blocks.push(BlockView {
                slug: block.slug.to_string(),
                title: block.title.clone(),
                context: block.context.clone(),
                files,
            });
        }

        let mut loose_skim = Vec::new();
        for entry in map.loose_skim() {
            if placed.contains(&entry.path) {
                continue;
            }
            let Some(change) = scope.change(&entry.path) else {
                continue;
            };
            placed.push(entry.path.clone());
            loose_skim.push(FileView::build(
                map,
                change,
                progress,
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

        let total_files = placed.len();
        let viewed_files = blocks
            .iter()
            .flat_map(|b| b.files.iter())
            .chain(loose_skim.iter())
            .filter(|f| f.viewed)
            .count();

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
        }
    }
}

impl CommentsView {
    pub fn of(found: &Found) -> Self {
        Self {
            comments: found.comments.iter().map(CommentView::of).collect(),
            unreadable: found
                .unreadable
                .iter()
                .map(|one| UnreadableView {
                    file: one.file.clone(),
                    about: one.about.clone(),
                    excerpt: one.excerpt.clone(),
                    why: says(one.why),
                })
                .collect(),
        }
    }
}

impl CommentView {
    pub fn of(comment: &Comment) -> Self {
        Self {
            id: comment.id.clone(),
            path: comment.path.clone(),
            from: comment.from,
            to: comment.to,
            body: comment.body.clone(),
            published: comment.published.clone(),
        }
    }
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
        skim: bool,
        skim_reason: Option<String>,
    ) -> FileView {
        let path = change.path.as_str();

        // Notes from every block the file appears in, so nothing is lost by
        // rendering it only once.
        let mut notes = Vec::new();
        let mut line_notes = Vec::new();
        for block in map.blocks_of(path) {
            if let Some(bf) = block.file(path) {
                if let Some(text) = &bf.note {
                    notes.push(TaggedNote {
                        block: block.slug.to_string(),
                        text: text.clone(),
                    });
                }
                for n in &bf.line_notes {
                    line_notes.push(TaggedLineNote {
                        block: block.slug.to_string(),
                        from: n.range.from,
                        to: n.range.to,
                        text: n.text.clone(),
                    });
                }
            }
        }
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
            notes,
            line_notes,
            tags: map
                .blocks_of(path)
                .iter()
                .map(|b| b.slug.to_string())
                .collect(),
            skim,
            skim_reason,
        }
    }
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
        ReviewView::build(&ReviewSnapshot {
            map: map.clone(),
            scope,
            progress: progress.clone(),
            commits_behind: 0,
        })
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
    fn a_file_in_two_blocks_is_rendered_once_in_the_earlier_one() {
        let map = map_of(&[("first", "shared.rs"), ("second", "shared.rs")]);
        let source = FakeDiffSource::with_paths(&["shared.rs"]);
        let view = built(&map, source, &Progress::new());

        assert_eq!(view.blocks[0].files.len(), 1);
        assert_eq!(view.blocks[1].files.len(), 0);
        assert_eq!(view.blocks[0].files[0].tags, vec!["first", "second"]);
        assert_eq!(view.total_files, 1);
    }

    #[test]
    fn notes_from_every_block_travel_with_the_single_rendering() {
        let mut map = map_of(&[("first", "shared.rs"), ("second", "shared.rs")]);
        map.update_file(
            &slug("first"),
            "shared.rs",
            Some("why it starts here".into()),
        )
        .unwrap();
        map.update_file(
            &slug("second"),
            "shared.rs",
            Some("why it matters again".into()),
        )
        .unwrap();
        map.add_line_note(
            &slug("second"),
            "shared.rs",
            LineRange::new(10, 12).unwrap(),
            "late note",
        )
        .unwrap();

        let source = FakeDiffSource::with_paths(&["shared.rs"]);
        let view = built(&map, source, &Progress::new());

        let file = &view.blocks[0].files[0];
        assert_eq!(file.notes.len(), 2);
        assert_eq!(file.notes[1].block, "second");
        assert_eq!(file.line_notes.len(), 1);
        assert_eq!(file.line_notes[0].block, "second");
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
    fn an_empty_comparison_leaves_the_blocks_standing_with_nothing_in_them() {
        // The map is not rewritten to repair the screen: the prose survives, the
        // file list does not.
        let map = map_of(&[("one", "a.rs"), ("two", "b.rs")]);

        let view = built(&map, FakeDiffSource::with_paths(&[]), &Progress::new());

        assert_eq!(view.blocks.len(), 2, "the map's own material is untouched");
        assert!(view.blocks.iter().all(|b| b.files.is_empty()));
        assert_eq!((view.total_files, view.viewed_files), (0, 0));
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
}
