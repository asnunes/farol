//! A thread's comment for the pull request, in the reviewer's pending review.

use clap::Subcommand;

use super::{Action, Ctx};
use crate::comments::domain::Author;
use crate::error::Result;

#[derive(Subcommand)]
pub(super) enum PendingAction {
    /// Write the thread's comment for the pull request. It goes into your
    /// pending review on GitHub, which only you see until the review is
    /// submitted, and replaces the one the thread had. Run from an agent
    /// session, it is signed by that harness.
    Write {
        id: String,
        #[arg(long, allow_hyphen_values = true)]
        text: String,
    },
    /// Change the words of a pending comment, by the GitHub id `farol github
    /// status` lists for it.
    Edit {
        id: String,
        #[arg(long, allow_hyphen_values = true)]
        text: String,
    },
    /// Take a pending comment back off the pull request, by its GitHub id.
    Remove { id: String },
}

impl Action for PendingAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            PendingAction::Write { id, text } => {
                let by = match super::wiring::harness_here() {
                    Some(name) => Author::Agent(name.to_string()),
                    None => Author::Reviewer,
                };
                ctx.write_pending.execute(&id, &text, by)?;
                println!("Wrote the pending comment of thread {id} to the pull request.");
            }
            PendingAction::Edit { id, text } => {
                ctx.edit_pending.execute(&id, &text)?;
                println!("Changed pending comment {id}.");
            }
            PendingAction::Remove { id } => {
                ctx.remove_pending.execute(&id)?;
                println!("Removed pending comment {id} from the pull request.");
            }
        }
        Ok(())
    }
}
