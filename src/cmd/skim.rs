//! Files the reviewer may read diagonally.

use clap::Subcommand;

use super::{Ctx, Reporting};
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
                ctx.map().require_in_scope(&path)?;
                ctx.edit(
                    || format!("Marked '{path}' as skim."),
                    |map| map.add_skim(&path, &reason, block),
                )
            }
            SkimAction::Remove { path } => ctx.edit(
                || format!("'{path}' is no longer marked as skim."),
                |map| map.remove_skim(&path),
            ),
        }
    }
}
