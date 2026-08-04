use clap::{Args, Parser, Subcommand};

mod wiring;

pub use wiring::Ctx;

use crate::diff::infra::ScopeRequest;
use crate::map::application::{ResetOutcome, position_from};
use crate::map::domain::{LineRange, ReviewMap};
use crate::map::presentation::{CheckSummary, MapReport, OrphanReport, ScopeReport};
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
    #[arg(long)]
    direct: bool,
    /// Include uncommitted changes from the working tree.
    #[arg(long)]
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
    #[arg(long)]
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

/// Reporting is the command layer's job, so it hangs off Ctx here rather than
/// inside the wiring.
trait Reporting {
    fn edit<F>(&self, done: impl FnOnce() -> String, edit: F) -> Result<()>
    where
        F: FnOnce(&mut ReviewMap) -> Result<()>;
}

impl Reporting for Ctx {
    fn edit<F>(&self, done: impl FnOnce() -> String, edit: F) -> Result<()>
    where
        F: FnOnce(&mut ReviewMap) -> Result<()>,
    {
        let map = self.map().edit(edit)?;
        println!("{}", done());
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
        #[command(subcommand)]
        action: MapAction,
    },
    /// Blocks: the units the reviewer reads in.
    Block {
        #[command(subcommand)]
        action: BlockAction,
    },
    /// Files inside a block.
    File {
        #[command(subcommand)]
        action: FileAction,
    },
    /// Notes pinned to a range of lines.
    Line {
        #[command(subcommand)]
        action: LineAction,
    },
    /// Files the reviewer may read diagonally.
    Skim {
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
                print!("{}", ScopeReport(ctx.source().scope()?));
                Ok(())
            }
            Command::Map { action } => action.run(),
            Command::Block { action } => action.run(),
            Command::File { action } => action.run(),
            Command::Line { action } => action.run(),
            Command::Skim { action } => action.run(),
        }
    }
}

#[derive(Args)]
pub struct ServeArgs {
    /// Base ref. Defaults to main, then master.
    base: Option<String>,
    /// Head ref. Defaults to the branch you are on.
    head: Option<String>,
    #[command(flatten)]
    scope: ScopeFlags,
    /// Port to listen on. 0 picks a free one.
    #[arg(long, default_value_t = 4600)]
    port: u16,
    /// Do not open a browser.
    #[arg(long)]
    no_open: bool,
    /// Do not watch the repository for changes.
    #[arg(long)]
    no_watch: bool,
}

impl ServeArgs {
    fn run(self) -> Result<()> {
        let scope = self.scope.with_base(self.base.clone());
        let ctx = Ctx::from_workspace(scope.request(self.head.clone()))?;

        // Without a map there is nothing farol can show. Falling back to a plain
        // diff viewer would make it a worse version of tools that already do
        // that well, so it refuses in the terminal instead of opening a browser
        // onto an apology.
        let map = ctx.map().require_current()?;

        crate::server::Server::new(crate::server::ServeConfig {
            source: ctx.source_arc(),
            progress: ctx.progress_arc(),
            map,
            port: self.port,
            open_browser: !self.no_open,
            watch: !self.no_watch,
            git_dir: ctx.git_dir().clone(),
        })
        .run()
    }
}

#[derive(Args)]
pub struct ScopeOnlyArgs {
    #[command(flatten)]
    scope: ScopeArgs,
}

#[derive(Subcommand)]
enum MapAction {
    /// Produce the map version for the current commit. Safe to call twice.
    Derive(ScopeOnlyArgs),
    /// Print the map.
    Show(ScopeOnlyArgs),
    /// Verify coverage and pending decisions. Non-zero exit when it fails.
    Check(ScopeOnlyArgs),
    /// Delete the newest version and fall back to the one before it.
    Reset(ScopeOnlyArgs),
}

impl MapAction {
    fn run(self) -> Result<()> {
        match self {
            MapAction::Derive(args) => {
                let ctx = args.scope.open()?;
                let session = ctx.map();
                let derived = session.derive()?;
                println!(
                    "{}",
                    if derived.created {
                        "Created the map version for this commit."
                    } else {
                        "A map version for this commit already exists — continuing from it."
                    }
                );
                print!(
                    "{}",
                    MapReport {
                        behind: session.behind(&derived.map),
                        map: &derived.map,
                    }
                );
                print!("{}", OrphanReport(&derived.map.orphans));
                Ok(())
            }
            MapAction::Show(args) => {
                let ctx = args.scope.open()?;
                let session = ctx.map();
                match session.current()? {
                    Some(map) => print!(
                        "{}",
                        MapReport {
                            behind: session.behind(&map),
                            map: &map,
                        }
                    ),
                    None => {
                        println!("No map for this branch yet. Run `farol map derive` to start one.")
                    }
                }
                Ok(())
            }
            MapAction::Check(args) => {
                let ctx = args.scope.open()?;
                let session = ctx.map();
                let report = session.check(&session.require_current()?)?;
                print!("{}", CheckSummary(&report));
                if report.passed() {
                    Ok(())
                } else {
                    std::process::exit(1)
                }
            }
            MapAction::Reset(args) => {
                let ctx = args.scope.open()?;
                match ctx.map().reset()? {
                    ResetOutcome::Deleted { fell_back_to } => {
                        println!("Deleted the map version for this commit.");
                        match fell_back_to {
                            Some(sha) => println!(
                                "The version from {} is current again.",
                                &sha[..7.min(sha.len())]
                            ),
                            None => {
                                println!("No earlier version remains — the branch is unmapped.")
                            }
                        }
                    }
                    ResetOutcome::NothingToDelete => {
                        println!("There is no map version for this commit to delete.")
                    }
                }
                Ok(())
            }
        }
    }
}

#[derive(Subcommand)]
enum BlockAction {
    Add {
        slug: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        context: String,
        /// Place ahead of this block.
        #[arg(long, conflicts_with = "after")]
        before: Option<String>,
        /// Place behind this block.
        #[arg(long)]
        after: Option<String>,
        /// Files with no note of their own can come along here.
        paths: Vec<String>,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Update {
        slug: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        context: Option<String>,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Remove {
        slug: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Move {
        slug: String,
        #[arg(long, conflicts_with = "after")]
        before: Option<String>,
        #[arg(long)]
        after: Option<String>,
        #[command(flatten)]
        scope: ScopeArgs,
    },
}

impl BlockAction {
    fn run(self) -> Result<()> {
        match self {
            BlockAction::Add {
                slug,
                title,
                context,
                before,
                after,
                paths,
                scope,
            } => {
                let ctx = scope.open()?;
                for path in &paths {
                    ctx.map().require_in_scope(path)?;
                }
                let position = position_from(before, after);
                let count = paths.len();
                ctx.edit(
                    || format!("Added block '{slug}' with {count} file(s)."),
                    |map| {
                        map.add_block(&slug, &title, &context, position)?;
                        for path in &paths {
                            map.add_file(&slug, path, None, None)?;
                        }
                        Ok(())
                    },
                )
            }
            BlockAction::Update {
                slug,
                title,
                context,
                scope,
            } => {
                let ctx = scope.open()?;
                ctx.edit(
                    || format!("Updated block '{slug}'."),
                    |map| map.update_block(&slug, title, context),
                )
            }
            BlockAction::Remove { slug, scope } => {
                let ctx = scope.open()?;
                ctx.edit(
                    || format!("Removed block '{slug}'."),
                    |map| map.remove_block(&slug),
                )
            }
            BlockAction::Move {
                slug,
                before,
                after,
                scope,
            } => {
                let ctx = scope.open()?;
                let position = position_from(before, after);
                ctx.edit(
                    || format!("Moved block '{slug}'."),
                    |map| map.move_block(&slug, position),
                )
            }
        }
    }
}

#[derive(Subcommand)]
enum FileAction {
    Add {
        slug: String,
        path: String,
        #[arg(long)]
        note: Option<String>,
        /// Place behind this file within the block.
        #[arg(long)]
        after: Option<String>,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Update {
        slug: String,
        path: String,
        #[arg(long)]
        note: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Remove {
        slug: String,
        path: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
}

impl FileAction {
    fn run(self) -> Result<()> {
        match self {
            FileAction::Add {
                slug,
                path,
                note,
                after,
                scope,
            } => {
                let ctx = scope.open()?;
                ctx.map().require_in_scope(&path)?;
                ctx.edit(
                    || format!("Added '{path}' to block '{slug}'."),
                    |map| map.add_file(&slug, &path, note, after.as_deref()),
                )
            }
            FileAction::Update {
                slug,
                path,
                note,
                scope,
            } => {
                let ctx = scope.open()?;
                ctx.edit(
                    || format!("Updated the note on '{path}'."),
                    |map| map.update_file(&slug, &path, Some(note)),
                )
            }
            FileAction::Remove { slug, path, scope } => {
                let ctx = scope.open()?;
                ctx.edit(
                    || format!("Removed '{path}' from block '{slug}'."),
                    |map| map.remove_file(&slug, &path),
                )
            }
        }
    }
}

#[derive(Subcommand)]
enum LineAction {
    Add {
        slug: String,
        path: String,
        /// Line range, for example 82-116.
        range: String,
        #[arg(long)]
        note: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Update {
        slug: String,
        path: String,
        range: String,
        #[arg(long)]
        note: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Remove {
        slug: String,
        path: String,
        range: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    /// Bring a deactivated note back at its new location.
    Restore {
        slug: String,
        path: String,
        /// The range it used to sit at, as reported by `map derive`.
        old_range: String,
        #[arg(long)]
        range: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    /// Drop a deactivated note for good.
    Discard {
        slug: String,
        path: String,
        old_range: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
}

impl LineAction {
    fn run(self) -> Result<()> {
        match self {
            LineAction::Add {
                slug,
                path,
                range,
                note,
                scope,
            } => {
                let ctx = scope.open()?;
                let range = LineRange::parse(&range)?;
                ctx.map().require_in_scope(&path)?;
                ctx.map().require_range_in_file(&path, range)?;
                ctx.edit(
                    || format!("Added a note on {path}:{range}."),
                    |map| map.add_line_note(&slug, &path, range, note),
                )
            }
            LineAction::Update {
                slug,
                path,
                range,
                note,
                scope,
            } => {
                let ctx = scope.open()?;
                let range = LineRange::parse(&range)?;
                ctx.edit(
                    || format!("Updated the note on {path}:{range}."),
                    |map| map.update_line_note(&slug, &path, range, note),
                )
            }
            LineAction::Remove {
                slug,
                path,
                range,
                scope,
            } => {
                let ctx = scope.open()?;
                let range = LineRange::parse(&range)?;
                ctx.edit(
                    || format!("Removed the note on {path}:{range}."),
                    |map| map.remove_line_note(&slug, &path, range),
                )
            }
            LineAction::Restore {
                slug,
                path,
                old_range,
                range,
                scope,
            } => {
                let ctx = scope.open()?;
                let old = LineRange::parse(&old_range)?;
                let new = LineRange::parse(&range)?;
                ctx.map().require_in_scope(&path)?;
                ctx.map().require_range_in_file(&path, new)?;
                ctx.edit(
                    || format!("Restored the note at {path}:{new}."),
                    |map| {
                        let orphan = map.take_orphan(&slug, &path, old)?;
                        map.add_line_note(&slug, &path, new, orphan.text)
                    },
                )
            }
            LineAction::Discard {
                slug,
                path,
                old_range,
                scope,
            } => {
                let ctx = scope.open()?;
                let range = LineRange::parse(&old_range)?;
                ctx.edit(
                    || format!("Discarded the note that was at {path}:{range}."),
                    |map| map.take_orphan(&slug, &path, range).map(|_| ()),
                )
            }
        }
    }
}

#[derive(Subcommand)]
enum SkimAction {
    Add {
        path: String,
        #[arg(long)]
        reason: String,
        /// Attach it to the block it belongs to, when it belongs to one.
        #[arg(long)]
        block: Option<String>,
        #[command(flatten)]
        scope: ScopeArgs,
    },
    Remove {
        path: String,
        #[command(flatten)]
        scope: ScopeArgs,
    },
}

impl SkimAction {
    fn run(self) -> Result<()> {
        match self {
            SkimAction::Add {
                path,
                reason,
                block,
                scope,
            } => {
                let ctx = scope.open()?;
                ctx.map().require_in_scope(&path)?;
                ctx.edit(
                    || format!("Marked '{path}' as skim."),
                    |map| map.add_skim(&path, &reason, block),
                )
            }
            SkimAction::Remove { path, scope } => {
                let ctx = scope.open()?;
                ctx.edit(
                    || format!("'{path}' is no longer marked as skim."),
                    |map| map.remove_skim(&path),
                )
            }
        }
    }
}
