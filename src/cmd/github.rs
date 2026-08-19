//! Sending the review to the pull request, from the terminal.
//!
//! The same two questions the browser asks, for a session driving farol without
//! one: can this go, and send it.

use clap::{Args, Subcommand};

use super::{Action, Ctx};
use crate::comments::domain::Verdict;
use crate::comments::presentation::Where;
use crate::error::Result;

#[derive(Subcommand)]
pub(super) enum GithubAction {
    /// Whether the review can be sent, and what is in the way when it cannot.
    /// Exits non-zero while it cannot.
    Status,
    /// Send the review to the pull request.
    Review(ReviewArgs),
}

impl Action for GithubAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            GithubAction::Status => {
                let standing = ctx.readiness.execute()?;
                print!("{}", Where(&standing));
                // The answer is on stdout either way, and the status says which
                // answer it was: a caller that only wants to know whether to go
                // on reads the code and never parses a word.
                if !standing.readiness.can_send() {
                    flush();
                    std::process::exit(1);
                }
                Ok(())
            }
            GithubAction::Review(args) => {
                let sent = ctx
                    .publish_review
                    .execute(args.verdict(), &args.summary.unwrap_or_default())?;
                let s = match sent.comments == 1 {
                    true => "",
                    false => "s",
                };
                println!("Sent {} comment{s}. {}", sent.comments, sent.url);
                Ok(())
            }
        }
    }
}

/// The verdict, and what the review says as a whole.
#[derive(Args)]
pub(super) struct ReviewArgs {
    #[command(flatten)]
    verdict: VerdictFlags,
    /// What the review says about the change as a whole. Markdown.
    #[arg(long)]
    summary: Option<String>,
}

/// Three flags that exclude each other, one of them required. Required because
/// the verdict is the decision a review is, and a default would be farol making
/// it on behalf of whoever ran the command.
#[derive(Args)]
#[group(required = true, multiple = false)]
struct VerdictFlags {
    /// Leave the comments without a verdict.
    #[arg(long)]
    comment: bool,
    /// Ask for the change to be reworked. Needs a summary.
    #[arg(long)]
    request_changes: bool,
    /// Say it is good to merge.
    #[arg(long)]
    approve: bool,
}

impl ReviewArgs {
    fn verdict(&self) -> Verdict {
        match (self.verdict.request_changes, self.verdict.approve) {
            (true, _) => Verdict::RequestChanges,
            (_, true) => Verdict::Approve,
            _ => Verdict::Comment,
        }
    }
}

fn flush() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
