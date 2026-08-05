//! Files the reviewer may read diagonally.

use clap::Subcommand;

use super::{Ctx, Reporting};
use crate::map::domain::Slug;
use crate::shared::error::Result;

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

impl SkimAction {
    pub(super) fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            SkimAction::Add {
                path,
                reason,
                block,
            } => {
                let path = ctx.map().review_path(&path)?;
                let block = block.map(|b| Slug::parse(&b)).transpose()?;
                ctx.report(
                    format!("Marked '{path}' as skim."),
                    ctx.map().add_skim(&path, &reason, block),
                )
            }
            SkimAction::Remove { path } => {
                let path = ctx.map().review_path(&path)?;
                ctx.report(
                    format!("'{path}' is no longer marked as skim."),
                    ctx.map().remove_skim(&path),
                )
            }
        }
    }
}
