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
    /// Everything still waiting for an answer.
    List,
    /// Answered, so it goes.
    Close { id: String },
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
            CommentAction::List => {
                print!("{}", CommentList(&ctx.comments.all()?));
                Ok(())
            }
            CommentAction::Close { id } => {
                ctx.comments.close(&id)?;
                println!("Closed comment {id}.");
                Ok(())
            }
        }
    }
}
