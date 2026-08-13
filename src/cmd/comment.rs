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
    List {
        /// Only the ones still waiting for an answer.
        #[arg(long)]
        open: bool,
    },
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
                // Only the range is parsed here: `<from>-<to>` is how the
                // terminal spells a span, and everything the comment is checked
                // against lives in the use case, where the browser reaches it
                // too.
                let range = LineRange::parse(&range)?;

                let one = ctx.comments.add(&path, range.from, range.to, &text)?;
                println!("Wrote comment {} on {}.", one.id, one.path);
                Ok(())
            }
            CommentAction::List { open } => {
                let all = ctx.comments.all()?;
                let shown: Vec<_> = match open {
                    true => all.into_iter().filter(|c| !c.resolved).collect(),
                    false => all,
                };
                print!("{}", CommentList(&shown));
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
