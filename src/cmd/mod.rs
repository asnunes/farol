use clap::{Args, Parser, Subcommand};

mod block;
mod file;
mod line;
mod map;
mod serve;
mod skim;
mod wiring;

use block::BlockAction;
use file::FileAction;
use line::LineAction;
use map::MapAction;
use serve::ServeArgs;
use skim::SkimAction;

pub use wiring::{Ctx, ServerUseCases};

use crate::diff::infra::ScopeRequest;
use crate::map::domain::ReviewMap;
use crate::map::presentation::{OrphanReport, ScopeReport};
use crate::shared::error::Result;

#[derive(Parser)]
#[command(
    name = "farol",
    about = "Read a branch in the order the person who wrote it would walk you through.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run() -> Result<()> {
        Self::parse().command.run()
    }
}

/// How to read the review window. `serve` takes its refs positionally, so the
/// base flag lives one level up rather than here — clap would otherwise see two
/// arguments called `base` on the same command.
#[derive(Args, Clone, Debug, Default)]
pub struct ScopeFlags {
    /// Diff the two refs directly instead of from their merge base.
    #[arg(long, global = true)]
    direct: bool,
    /// Include uncommitted changes from the working tree.
    #[arg(long, global = true)]
    dirty: bool,
}

impl ScopeFlags {
    fn with_base(&self, base: Option<String>) -> ScopeArgs {
        ScopeArgs {
            base,
            flags: self.clone(),
        }
    }
}

/// The window, for commands with no positional slots to spare.
#[derive(Args, Clone, Debug, Default)]
pub struct ScopeArgs {
    /// Compare against this ref instead of main (falling back to master).
    #[arg(long, global = true)]
    base: Option<String>,
    #[command(flatten)]
    flags: ScopeFlags,
}

impl ScopeArgs {
    fn request(&self, head: Option<String>) -> ScopeRequest {
        ScopeRequest {
            base: self.base.clone(),
            head,
            direct: self.flags.direct,
            dirty: self.flags.dirty,
        }
    }

    fn open(&self) -> Result<Ctx> {
        Ctx::from_workspace(self.request(None))
    }
}

/// Printing is the entry point's job: the use cases return the map they
/// produced, and this turns it into what the terminal sees. Anything a use case
/// deactivated is reported, because a note that vanished without a word is
/// exactly the failure the orphan machinery exists to prevent.
pub(super) trait Reporting {
    fn report(&self, done: String, outcome: Result<ReviewMap>) -> Result<()>;
}

impl Reporting for Ctx {
    fn report(&self, done: String, outcome: Result<ReviewMap>) -> Result<()> {
        let map = outcome?;
        println!("{done}");
        print!("{}", OrphanReport(&map.orphans));
        Ok(())
    }
}

#[derive(Subcommand)]
enum Command {
    /// Serve the review in a browser.
    Serve(ServeArgs),
    /// List the files under review, as farol sees them.
    Scope(ScopeOnlyArgs),
    /// Create, inspect and verify the map.
    Map {
        #[command(flatten)]
        scope: ScopeArgs,
        #[command(subcommand)]
        action: MapAction,
    },
    /// Blocks: the units the reviewer reads in.
    Block {
        #[command(flatten)]
        scope: ScopeArgs,
        #[command(subcommand)]
        action: BlockAction,
    },
    /// Files inside a block.
    File {
        #[command(flatten)]
        scope: ScopeArgs,
        #[command(subcommand)]
        action: FileAction,
    },
    /// Notes pinned to a range of lines.
    Line {
        #[command(flatten)]
        scope: ScopeArgs,
        #[command(subcommand)]
        action: LineAction,
    },
    /// Files the reviewer may read diagonally.
    Skim {
        #[command(flatten)]
        scope: ScopeArgs,
        #[command(subcommand)]
        action: SkimAction,
    },
}

impl Command {
    fn run(self) -> Result<()> {
        match self {
            Command::Serve(args) => args.run(),
            Command::Scope(args) => {
                let ctx = args.scope.open()?;
                print!("{}", ScopeReport(&ctx.scope.execute()?));
                Ok(())
            }
            // The window is resolved once per group and handed down, instead
            // of every action declaring and reopening it.
            Command::Map { scope, action } => action.run(&scope.open()?),
            Command::Block { scope, action } => action.run(&scope.open()?),
            Command::File { scope, action } => action.run(&scope.open()?),
            Command::Line { scope, action } => action.run(&scope.open()?),
            Command::Skim { scope, action } => action.run(&scope.open()?),
        }
    }
}

#[derive(Args)]
pub struct ScopeOnlyArgs {
    #[command(flatten)]
    scope: ScopeArgs,
}
