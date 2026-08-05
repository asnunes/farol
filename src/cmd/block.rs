//! Blocks: the units the reviewer reads in.

use clap::Subcommand;

use super::{Ctx, Reporting};
use crate::map::application::position_from;
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
                paths,
            } => {
                for path in &paths {
                    ctx.map().require_in_scope(path)?;
                }
                let position = position_from(before, after);
                let count = paths.len();
                ctx.edit(
                    || format!("Added block '{slug}' with {count} file(s)."),
                    |map| {
                        map.add_block(&slug, &title, &context, position)?;
                        for path in &paths {
                            map.add_file(&slug, path, None, None)?;
                        }
                        Ok(())
                    },
                )
            }
            BlockAction::Update {
                slug,
                title,
                context,
            } => ctx.edit(
                || format!("Updated block '{slug}'."),
                |map| map.update_block(&slug, title, context),
            ),
            BlockAction::Remove { slug } => ctx.edit(
                || format!("Removed block '{slug}'."),
                |map| map.remove_block(&slug),
            ),
            BlockAction::Move {
                slug,
                before,
                after,
            } => {
                let position = position_from(before, after);
                ctx.edit(
                    || format!("Moved block '{slug}'."),
                    |map| map.move_block(&slug, position),
                )
            }
        }
    }
}
