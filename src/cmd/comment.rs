//! What the reviewer wrote back.

use clap::Subcommand;

use super::{Action, Ctx};
use crate::comments::presentation::CommentList;
use crate::error::Result;
use crate::map::domain::LineRange;

#[derive(Subcommand)]
pub(super) enum CommentAction {
    Add {
        path: String,
        /// Line range, for example 82-116.
        range: String,
        #[arg(long)]
        text: String,
    },
    List,
    /// Close one, because it was answered.
    Resolve {
        id: String,
    },
    /// Open one again.
    Reopen {
        id: String,
    },
    Remove {
        id: String,
    },
}

impl Action for CommentAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            CommentAction::Add { path, range, text } => {
                // Through the scope, so a comment cannot be left on a file the
                // reviewer is not looking at, and a range past the end of the
                // file is refused where it is written rather than on screen.
                let path = ctx.scope.path(&path)?;
                let range = LineRange::parse(&range)?;
                range.require_within(&path)?;

                let one = ctx
                    .comments
                    .add(path.as_str(), range.from, range.to, &text)?;
                println!("Wrote comment {} on {}.", one.id, one.path);
                Ok(())
            }
            CommentAction::List => {
                print!("{}", CommentList(&ctx.comments.all()?));
                Ok(())
            }
            CommentAction::Resolve { id } => {
                ctx.comments.resolve(&id, true)?;
                println!("Closed comment {id}.");
                Ok(())
            }
            CommentAction::Reopen { id } => {
                ctx.comments.resolve(&id, false)?;
                println!("Reopened comment {id}.");
                Ok(())
            }
            CommentAction::Remove { id } => {
                ctx.comments.remove(&id)?;
                println!("Removed comment {id}.");
                Ok(())
            }
        }
    }
}
