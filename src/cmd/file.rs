//! Files inside a block.

use clap::Subcommand;

use super::{Ctx, Reporting};
use crate::map::domain::Slug;
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
                let slug = Slug::parse(&slug)?;
                let path = ctx.scope.path(&path)?;
                // `--after` names a file already in the block, which is under
                // review by definition.
                let after = after.map(|a| ctx.scope.path(&a)).transpose()?;
                ctx.report(
                    format!("Added '{path}' to block '{slug}'."),
                    ctx.add_file.execute(&slug, &path, note, after.as_ref()),
                )
            }
            FileAction::Update { slug, path, note } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.scope.path(&path)?;
                ctx.report(
                    format!("Updated the note on '{path}'."),
                    ctx.update_file.execute(&slug, &path, note),
                )
            }
            FileAction::Remove { slug, path } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.scope.path(&path)?;
                ctx.report(
                    format!("Removed '{path}' from block '{slug}'."),
                    ctx.remove_file.execute(&slug, &path),
                )
            }
        }
    }
}
