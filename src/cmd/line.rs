//! Notes pinned to a range of lines, and the deactivated ones.

use clap::Subcommand;

use super::{Ctx, Reporting};
use crate::map::domain::{LineRange, Slug};
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
                let slug = Slug::parse(&slug)?;
                let path = ctx.map().review_path(&path)?;
                let range = LineRange::parse(&range)?;
                ctx.report(
                    format!("Added a note on {path}:{range}."),
                    ctx.map().add_line_note(&slug, &path, range, note),
                )
            }
            LineAction::Update {
                slug,
                path,
                range,
                note,
            } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.map().review_path(&path)?;
                let range = LineRange::parse(&range)?;
                ctx.report(
                    format!("Updated the note on {path}:{range}."),
                    ctx.map().update_line_note(&slug, &path, range, note),
                )
            }
            LineAction::Remove { slug, path, range } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.map().review_path(&path)?;
                let range = LineRange::parse(&range)?;
                ctx.report(
                    format!("Removed the note on {path}:{range}."),
                    ctx.map().remove_line_note(&slug, &path, range),
                )
            }
            LineAction::Restore {
                slug,
                path,
                old_range,
                range,
            } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.map().review_path(&path)?;
                let old = LineRange::parse(&old_range)?;
                let new = LineRange::parse(&range)?;
                ctx.report(
                    format!("Restored the note at {path}:{new}."),
                    ctx.map().restore_note(&slug, &path, old, new),
                )
            }
            LineAction::Discard {
                slug,
                path,
                old_range,
            } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.map().review_path(&path)?;
                let old = LineRange::parse(&old_range)?;
                ctx.report(
                    format!("Discarded the note that was at {path}:{old}."),
                    ctx.map().discard_note(&slug, &path, old),
                )
            }
        }
    }
}
