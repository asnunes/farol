//! Files inside a block.

use clap::Subcommand;

use super::{Action, Ctx, Reporting};
use crate::error::Result;
use crate::map::domain::Slug;

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
    /// Mark a file worth a closer look, saying why; it starts open on screen.
    Look {
        slug: String,
        path: String,
        #[arg(long)]
        reason: String,
    },
    /// Take the closer-look mark off a file.
    Unlook {
        slug: String,
        path: String,
    },
}

impl Action for FileAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
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
            FileAction::Look { slug, path, reason } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.scope.path(&path)?;
                ctx.report(
                    format!("Marked '{path}' worth a closer look."),
                    ctx.look_file.execute(&slug, &path, Some(reason)),
                )
            }
            FileAction::Unlook { slug, path } => {
                let slug = Slug::parse(&slug)?;
                let path = ctx.scope.path(&path)?;
                ctx.report(
                    format!("Took the closer-look mark off '{path}'."),
                    ctx.look_file.execute(&slug, &path, None),
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
