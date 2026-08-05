//! Files inside a block.

use clap::Subcommand;

use super::{Ctx, Reporting};
use crate::shared::error::Result;

#[derive(Subcommand)]
pub(super) enum FileAction {
    Add {
        slug: String,
        path: String,
        #[arg(long)]
        note: Option<String>,
        /// Place behind this file within the block.
        #[arg(long)]
        after: Option<String>,
    },
    Update {
        slug: String,
        path: String,
        #[arg(long)]
        note: String,
    },
    Remove {
        slug: String,
        path: String,
    },
}

impl FileAction {
    pub(super) fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            FileAction::Add {
                slug,
                path,
                note,
                after,
            } => {
                ctx.map().require_in_scope(&path)?;
                ctx.edit(
                    || format!("Added '{path}' to block '{slug}'."),
                    |map| map.add_file(&slug, &path, note, after.as_deref()),
                )
            }
            FileAction::Update { slug, path, note } => ctx.edit(
                || format!("Updated the note on '{path}'."),
                |map| map.update_file(&slug, &path, Some(note)),
            ),
            FileAction::Remove { slug, path } => ctx.edit(
                || format!("Removed '{path}' from block '{slug}'."),
                |map| map.remove_file(&slug, &path),
            ),
        }
    }
}
