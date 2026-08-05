//! Blocks: the units the reviewer reads in.

use clap::Subcommand;

use super::{Ctx, Reporting};
use crate::diff::domain as paths;
use crate::map::application::position_from;
use crate::map::domain::Slug;
use crate::shared::error::Result;

#[derive(Subcommand)]
pub(super) enum BlockAction {
    Add {
        slug: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        context: String,
        /// Place ahead of this block.
        #[arg(long, conflicts_with = "after")]
        before: Option<String>,
        /// Place behind this block.
        #[arg(long)]
        after: Option<String>,
        /// Files with no note of their own can come along here.
        paths: Vec<String>,
    },
    Update {
        slug: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        context: Option<String>,
    },
    Remove {
        slug: String,
    },
    Move {
        slug: String,
        #[arg(long, conflicts_with = "after")]
        before: Option<String>,
        #[arg(long)]
        after: Option<String>,
    },
}

impl BlockAction {
    pub(super) fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            BlockAction::Add {
                slug,
                title,
                context,
                before,
                after,
                paths: raw,
            } => {
                // Raw strings become proven values here, at the edge. Past this
                // point the use case cannot be handed anything unchecked.
                let slug = Slug::parse(&slug)?;
                let files = paths::all(ctx.source(), &raw)?;
                ctx.report(
                    format!("Added block '{slug}' with {} file(s).", files.len()),
                    ctx.map().add_block(
                        &slug,
                        &title,
                        &context,
                        position_from(before, after),
                        &files,
                    ),
                )
            }
            BlockAction::Update {
                slug,
                title,
                context,
            } => {
                let slug = Slug::parse(&slug)?;
                ctx.report(
                    format!("Updated block '{slug}'."),
                    ctx.map().update_block(&slug, title, context),
                )
            }
            BlockAction::Remove { slug } => {
                let slug = Slug::parse(&slug)?;
                ctx.report(
                    format!("Removed block '{slug}'."),
                    ctx.map().remove_block(&slug),
                )
            }
            BlockAction::Move {
                slug,
                before,
                after,
            } => {
                let slug = Slug::parse(&slug)?;
                ctx.report(
                    format!("Moved block '{slug}'."),
                    ctx.map().move_block(&slug, position_from(before, after)),
                )
            }
        }
    }
}
