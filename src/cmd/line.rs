//! Notes pinned to a range of lines, and the deactivated ones.

use clap::Subcommand;

use super::{Ctx, Reporting};
use crate::map::domain::LineRange;
use crate::shared::error::Result;

#[derive(Subcommand)]
pub(super) enum LineAction {
    Add {
        slug: String,
        path: String,
        /// Line range, for example 82-116.
        range: String,
        #[arg(long)]
        note: String,
    },
    Update {
        slug: String,
        path: String,
        range: String,
        #[arg(long)]
        note: String,
    },
    Remove {
        slug: String,
        path: String,
        range: String,
    },
    /// Bring a deactivated note back at its new location.
    Restore {
        slug: String,
        path: String,
        /// The range it used to sit at, as reported by `map derive`.
        old_range: String,
        #[arg(long)]
        range: String,
    },
    /// Drop a deactivated note for good.
    Discard {
        slug: String,
        path: String,
        old_range: String,
    },
}

impl LineAction {
    pub(super) fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            LineAction::Add {
                slug,
                path,
                range,
                note,
            } => {
                let range = LineRange::parse(&range)?;
                ctx.map().require_in_scope(&path)?;
                ctx.map().require_range_in_file(&path, range)?;
                ctx.edit(
                    || format!("Added a note on {path}:{range}."),
                    |map| map.add_line_note(&slug, &path, range, note),
                )
            }
            LineAction::Update {
                slug,
                path,
                range,
                note,
            } => {
                let range = LineRange::parse(&range)?;
                ctx.edit(
                    || format!("Updated the note on {path}:{range}."),
                    |map| map.update_line_note(&slug, &path, range, note),
                )
            }
            LineAction::Remove { slug, path, range } => {
                let range = LineRange::parse(&range)?;
                ctx.edit(
                    || format!("Removed the note on {path}:{range}."),
                    |map| map.remove_line_note(&slug, &path, range),
                )
            }
            LineAction::Restore {
                slug,
                path,
                old_range,
                range,
            } => {
                let old = LineRange::parse(&old_range)?;
                let new = LineRange::parse(&range)?;
                ctx.map().require_in_scope(&path)?;
                ctx.map().require_range_in_file(&path, new)?;
                ctx.edit(
                    || format!("Restored the note at {path}:{new}."),
                    |map| {
                        let orphan = map.take_orphan(&slug, &path, old)?;
                        map.add_line_note(&slug, &path, new, orphan.text)
                    },
                )
            }
            LineAction::Discard {
                slug,
                path,
                old_range,
            } => {
                let range = LineRange::parse(&old_range)?;
                ctx.edit(
                    || format!("Discarded the note that was at {path}:{range}."),
                    |map| map.take_orphan(&slug, &path, range).map(|_| ()),
                )
            }
        }
    }
}
