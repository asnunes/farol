//! Files the reviewer may read diagonally.

use clap::Subcommand;

use super::{Action, Ctx, Reporting};
use crate::error::Result;
use crate::map::domain::Slug;

#[derive(Subcommand)]
pub(super) enum SkimAction {
    Add {
        path: String,
        #[arg(long)]
        reason: String,
        /// Attach it to the block it belongs to, when it belongs to one.
        #[arg(long)]
        block: Option<String>,
    },
    Remove {
        path: String,
    },
}

impl Action for SkimAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            SkimAction::Add {
                path,
                reason,
                block,
            } => {
                let path = ctx.scope.path(&path)?;
                let block = block.map(|b| Slug::parse(&b)).transpose()?;
                ctx.report(
                    format!("Marked '{path}' as skim."),
                    ctx.add_skim.execute(&path, &reason, block),
                )
            }
            SkimAction::Remove { path } => {
                let path = ctx.scope.path(&path)?;
                ctx.report(
                    format!("'{path}' is no longer marked as skim."),
                    ctx.remove_skim.execute(&path),
                )
            }
        }
    }
}
