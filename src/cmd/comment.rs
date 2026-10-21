//! What the reviewer wrote back.

use clap::Subcommand;

use super::{Action, Ctx};
use crate::comments::domain::Author;
use crate::comments::presentation::CommentList;
use crate::diff::domain::Side;
use crate::error::Result;
use crate::map::domain::LineRange;

#[derive(Subcommand)]
pub(super) enum CommentAction {
    Add {
        path: String,
        /// Line range, for example 82-116.
        range: String,
        /// Which side of the diff the range is counted on: `new` is the file as
        /// it now reads, `old` is what the change took away. They number
        /// separately, so 82 on one is not 82 on the other.
        #[arg(long, default_value = "new", value_parser = side_from)]
        side: Side,
        #[arg(long)]
        text: String,
    },
    /// Everything still waiting for an answer.
    List,
    /// Answered, so it goes.
    Close { id: String },
    /// Say you have started on a comment sent from farol, so the reviewer sees it.
    Ack { id: String },
    /// Answer a comment. Run from an agent session, it is signed by that
    /// harness; from a plain terminal, by the reviewer.
    Reply {
        id: String,
        #[arg(long, allow_hyphen_values = true)]
        text: String,
    },
}

impl Action for CommentAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            CommentAction::Add {
                path,
                range,
                side,
                text,
            } => {
                // Only the range is parsed here: `<from>-<to>` is how the
                // terminal spells a span, and everything the comment is checked
                // against lives in the use case, where the browser reaches it
                // too. The side arrived proven, parsed by clap on the way in.
                let range = LineRange::parse(&range)?;

                let one = ctx.comments.add(&path, side, range.from, range.to, &text)?;
                println!("Wrote comment {} on {}.", one.id, one.at());
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
            CommentAction::Ack { id } => {
                let by = super::wiring::harness_here().unwrap_or("agent");
                ctx.comments.acknowledge(&id, by)?;
                println!("Marked comment {id} as being worked on by {by}.");
                Ok(())
            }
            CommentAction::Reply { id, text } => {
                let from = match super::wiring::harness_here() {
                    Some(name) => Author::Agent(name.to_string()),
                    None => Author::Reviewer,
                };
                ctx.comments.reply(&id, from, &text)?;
                println!("Replied to comment {id}.");
                Ok(())
            }
        }
    }
}

/// The flag as a side, refused in clap's own words when it is neither.
///
/// A free function for the same reason `position_from` is one: it turns raw
/// input into a proven value and belongs to neither the parser nor the domain.
/// Handed to clap rather than checked after, so `--side sideways` never reaches
/// a use case and the list of what is allowed is written once.
fn side_from(raw: &str) -> std::result::Result<Side, String> {
    Side::parse(raw).ok_or_else(|| format!("expected 'old' or 'new', not '{raw}'"))
}
