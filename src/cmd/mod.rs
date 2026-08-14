use clap::{Args, Parser, Subcommand};

mod block;
mod comment;
mod file;
mod line;
mod map;
mod scope;
mod serve;
mod servers;
mod skim;
mod wiring;

use block::BlockAction;
use comment::CommentAction;
use file::FileAction;
use line::LineAction;
use map::MapAction;
use scope::ShowScope;
use serve::ServeArgs;
use servers::ServersArgs;
use skim::SkimAction;

pub use wiring::{Ctx, ServerUseCases};

use crate::diff::infra::ScopeRequest;
use crate::error::Result;
use crate::map::domain::ReviewMap;
use crate::map::presentation::OrphanReport;

#[derive(Parser)]
#[command(
    name = "farol",
    about = "A walkthrough, not a diff: the author's order, and the reasons behind it.",
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
    /// Whether the window was left alone. `serve` asks so it can tell a plain
    /// start from one that asked for something in particular.
    fn is_default(&self) -> bool {
        !self.direct && !self.dirty
    }

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

/// What every command group is: something you can run once the review window
/// has been resolved. Stated as a trait so the shape is enforced rather than
/// merely repeated.
pub(super) trait Action {
    fn run(self, ctx: &Ctx) -> Result<()>;
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
        print!("{}", OrphanReport(map.orphans()));
        Ok(())
    }
}

#[derive(Subcommand)]
enum Command {
    /// Serve the review in a browser.
    Serve(ServeArgs),
    /// The reviews open on this machine.
    Servers(ServersArgs),
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
    /// What the reviewer wrote back.
    Comment {
        #[command(flatten)]
        scope: ScopeArgs,
        #[command(subcommand)]
        action: CommentAction,
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
            // Serve is the one that does not fit: it takes its refs
            // positionally and hands the use cases to a long-lived server.
            Command::Serve(args) => args.run(),

            // And `servers` asks about the machine rather than a repository:
            // it answers from anywhere, including outside a git repository.
            Command::Servers(args) => args.run(),

            // The rest are uniform. The window is resolved once per group and
            // handed down, instead of every action reopening it. The match
            // stays a match on purpose: it is exhaustive, so a new group that
            // forgets its arm does not compile — which is the registration
            // mistake a dispatch table would need Open/Closed to guard against.
            Command::Scope(args) => ShowScope.run(&args.scope.open()?),
            Command::Map { scope, action } => action.run(&scope.open()?),
            Command::Block { scope, action } => action.run(&scope.open()?),
            Command::File { scope, action } => action.run(&scope.open()?),
            Command::Line { scope, action } => action.run(&scope.open()?),
            Command::Skim { scope, action } => action.run(&scope.open()?),
            Command::Comment { scope, action } => action.run(&scope.open()?),
        }
    }
}

#[derive(Args)]
pub struct ScopeOnlyArgs {
    #[command(flatten)]
    scope: ScopeArgs,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("farol").chain(args.iter().copied()))
            .unwrap_or_else(|e| panic!("farol {args:?} should parse:\n{e}"))
    }

    fn refuses(args: &[&str]) {
        assert!(
            Cli::try_parse_from(std::iter::once("farol").chain(args.iter().copied())).is_err(),
            "farol {args:?} should have been refused"
        );
    }

    /// clap catches conflicting flag names and duplicate argument ids only at
    /// runtime; without this the binary would panic on first use.
    #[test]
    fn the_command_surface_is_internally_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn the_window_flags_reach_every_group() {
        // They are global so `farol block add … --dirty` works, rather than
        // making the author remember which groups accept them.
        for group in [
            vec!["scope", "--dirty"],
            vec!["map", "show", "--dirty"],
            vec!["block", "remove", "core", "--direct"],
            vec!["skim", "remove", "Cargo.lock", "--base", "develop"],
        ] {
            parse(&group);
        }
    }

    #[test]
    fn the_window_flags_become_the_request_that_opens_the_scope() {
        let args = ScopeArgs {
            base: Some("develop".into()),
            flags: ScopeFlags {
                direct: true,
                dirty: true,
            },
        };

        let req = args.request(Some("feature/x".into()));

        assert_eq!(req.base.as_deref(), Some("develop"));
        assert_eq!(req.head.as_deref(), Some("feature/x"));
        assert!(req.direct);
        assert!(req.dirty);
    }

    #[test]
    fn asking_for_nothing_in_particular_asks_for_the_default_window() {
        let req = ScopeArgs::default().request(None);

        assert_eq!(req.base, None, "main, then master, is decided further down");
        assert_eq!(req.head, None);
        assert!(!req.direct);
        assert!(!req.dirty);
    }

    #[test]
    fn serve_takes_its_refs_positionally_and_the_base_flag_still_works() {
        // The reason `--base` lives one level up: two arguments called `base`
        // on the same command is the mistake this shape avoids.
        parse(&["serve"]);
        parse(&["serve", "main"]);
        parse(&["serve", "main", "feature/x"]);
        parse(&["scope", "--base", "main"]);
    }

    #[test]
    fn every_group_refuses_a_subcommand_it_does_not_have() {
        refuses(&["block", "sprinkle", "core"]);
        refuses(&["map", "publish"]);
        refuses(&["nonsense"]);
    }

    #[test]
    fn commands_that_name_a_block_require_one() {
        refuses(&["block", "remove"]);
        refuses(&["file", "remove", "core"]);
        refuses(&["line", "remove", "core", "src/a.rs"]);
    }

    #[test]
    fn adding_a_block_requires_the_prose_that_makes_it_worth_reading() {
        // A block with no context is a heading, and the map exists for what is
        // under the heading.
        refuses(&["block", "add", "core"]);
        refuses(&["block", "add", "core", "--title", "t"]);
        parse(&["block", "add", "core", "--title", "t", "--context", "c"]);
    }

    #[test]
    fn a_block_cannot_be_placed_before_and_after_at_once() {
        refuses(&[
            "block",
            "add",
            "core",
            "--title",
            "t",
            "--context",
            "c",
            "--before",
            "a",
            "--after",
            "b",
        ]);
    }
}
