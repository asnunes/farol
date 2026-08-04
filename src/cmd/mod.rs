use clap::{Args, Parser, Subcommand};

use crate::diff::domain::DiffSource;
use crate::diff::infra::{GixSource, ScopeRequest};
use crate::map::application::{
    MapWriter, check, derive, parse_range, position_from, require_in_scope, require_range_in_file,
};
use crate::map::domain::{MapRepository, ReviewMap, WORKING};
use crate::map::infra::JsonMapRepository;
use crate::map::presentation::{render_check, render_map, render_orphans, render_scope};
use crate::progress::infra::JsonProgressRepository;
use crate::shared::error::{Error, Result};
use crate::shared::paths::{Store, current_branch, discover};

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

/// Everything `serve` takes positionally, for the commands that have no
/// positional slots to spare.
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
}

impl ScopeFlags {
    fn with_base(&self, base: Option<String>) -> ScopeArgs {
        ScopeArgs {
            base,
            flags: self.clone(),
        }
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

/// Everything a command needs, assembled once.
struct Ctx {
    source: GixSource,
    map_repo: JsonMapRepository,
    git_dir: std::path::PathBuf,
    branch: String,
}

fn open(scope: &ScopeArgs, base: Option<String>, head: Option<String>) -> Result<Ctx> {
    let cwd = std::env::current_dir()?;
    let repo = discover(&cwd)?;
    let branch = current_branch(&repo)?;
    // The worktree's own git dir, asked of git rather than assembled by hand:
    // inside a worktree `.git` is a file, and joining onto it would fail.
    let git_dir = repo.git_dir().to_path_buf();
    let mut request = scope.request(head);
    if request.base.is_none() {
        request.base = base;
    }
    let source = GixSource::open(repo, &request)?;
    Ok(Ctx {
        source,
        map_repo: JsonMapRepository::new(Store::new(&git_dir, &branch)),
        git_dir,
        branch,
    })
}

impl Ctx {
    fn writer(&self) -> MapWriter<'_> {
        MapWriter {
            source: &self.source,
            repo: &self.map_repo,
        }
    }

    fn store(&self) -> Store {
        Store::new(&self.git_dir, &self.branch)
    }

    /// The map that belongs to where we are now, or nothing at all.
    fn current_map(&self) -> Result<Option<ReviewMap>> {
        let scope = self.source.scope()?;
        let target = if scope.dirty {
            WORKING.to_string()
        } else {
            scope.head_sha.clone()
        };
        if let Some(map) = self.map_repo.load_at(&target)? {
            return Ok(Some(map));
        }
        // Fall back to the newest ancestor that has one, which is what makes
        // "the map is 2 commits behind" a state instead of an absence.
        let mut best: Option<(u32, ReviewMap)> = None;
        for sha in self.map_repo.stored_shas()? {
            if sha == WORKING || !self.source.is_ancestor(&sha)? {
                continue;
            }
            let distance = self.source.commits_ahead_of(&sha)?;
            if best.as_ref().map(|(d, _)| distance < *d).unwrap_or(true)
                && let Some(map) = self.map_repo.load_at(&sha)?
            {
                best = Some((distance, map));
            }
        }
        Ok(best.map(|(_, m)| m))
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve(args) => serve(args),
        Command::Scope(args) => {
            let ctx = open(&args.scope, None, None)?;
            print!("{}", render_scope(ctx.source.scope()?));
            Ok(())
        }
        Command::Map { action } => map(action),
        Command::Block { action } => block(action),
        Command::File { action } => file(action),
        Command::Line { action } => line(action),
        Command::Skim { action } => skim(action),
    }
}

fn serve(args: ServeArgs) -> Result<()> {
    let scope = args.scope.with_base(args.base.clone());
    let ctx = open(&scope, None, args.head.clone())?;

    // Without a map there is nothing farol can show. Falling back to a plain
    // diff viewer would make it a worse version of tools that already do that
    // well, so it refuses in the terminal instead of opening a browser onto an
    // apology.
    let Some(map) = ctx.current_map()? else {
        return Err(Error::NoMap {
            branch: ctx.source.scope()?.branch.clone(),
        });
    };

    let store = ctx.store();
    crate::server::serve(crate::server::ServeConfig {
        source: ctx.source,
        progress: JsonProgressRepository::new(store),
        map,
        port: args.port,
        open_browser: !args.no_open,
        watch: !args.no_watch,
        git_dir: ctx.git_dir,
    })
}

fn map(action: MapAction) -> Result<()> {
    match action {
        MapAction::Derive(args) => {
            let ctx = open(&args.scope, None, None)?;
            let derived = derive(&ctx.source, &ctx.map_repo)?;
            let behind = ctx
                .source
                .commits_ahead_of(&derived.map.generated_at)
                .unwrap_or(0);
            if derived.created {
                println!("Created the map version for this commit.");
            } else {
                println!("A map version for this commit already exists — continuing from it.");
            }
            print!("{}", render_map(&derived.map, behind));
            print!("{}", render_orphans(&derived.map.orphans));
            Ok(())
        }
        MapAction::Show(args) => {
            let ctx = open(&args.scope, None, None)?;
            let Some(map) = ctx.current_map()? else {
                println!("No map for this branch yet. Run `farol map derive` to start one.");
                return Ok(());
            };
            let behind = ctx.source.commits_ahead_of(&map.generated_at).unwrap_or(0);
            print!("{}", render_map(&map, behind));
            Ok(())
        }
        MapAction::Check(args) => {
            let ctx = open(&args.scope, None, None)?;
            let Some(map) = ctx.current_map()? else {
                return Err(Error::NoMap {
                    branch: ctx.source.scope()?.branch.clone(),
                });
            };
            let report = check(&map, &ctx.source)?;
            print!("{}", render_check(&report));
            if report.passed() {
                Ok(())
            } else {
                std::process::exit(1);
            }
        }
        MapAction::Reset(args) => {
            let ctx = open(&args.scope, None, None)?;
            let scope = ctx.source.scope()?;
            let target = if scope.dirty {
                WORKING.to_string()
            } else {
                scope.head_sha.clone()
            };
            match ctx.map_repo.load_at(&target)? {
                Some(_) => {
                    ctx.map_repo.delete(&target)?;
                    println!("Deleted the map version for this commit.");
                    match ctx.current_map()? {
                        Some(prev) => println!(
                            "The version from {} is current again.",
                            &prev.generated_at[..7.min(prev.generated_at.len())]
                        ),
                        None => println!("No earlier version remains — the branch is unmapped."),
                    }
                }
                None => println!("There is no map version for this commit to delete."),
            }
            Ok(())
        }
    }
}

fn block(action: BlockAction) -> Result<()> {
    match action {
        BlockAction::Add {
            slug,
            title,
            context,
            before,
            after,
            paths,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            for p in &paths {
                require_in_scope(&ctx.source, p)?;
            }
            let position = position_from(before, after);
            ctx.writer().edit(|map, _| {
                map.add_block(&slug, &title, &context, position)?;
                for p in &paths {
                    map.add_file(&slug, p, None, None)?;
                }
                Ok(())
            })?;
            println!("Added block '{slug}' with {} file(s).", paths.len());
            Ok(())
        }
        BlockAction::Update {
            slug,
            title,
            context,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            ctx.writer()
                .edit(|map, _| map.update_block(&slug, title, context))?;
            println!("Updated block '{slug}'.");
            Ok(())
        }
        BlockAction::Remove { slug, scope } => {
            let ctx = open(&scope, None, None)?;
            let map = ctx.writer().edit(|map, _| map.remove_block(&slug))?;
            println!("Removed block '{slug}'.");
            print!("{}", render_orphans(&map.orphans));
            Ok(())
        }
        BlockAction::Move {
            slug,
            before,
            after,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            let position = position_from(before, after);
            ctx.writer()
                .edit(|map, _| map.move_block(&slug, position))?;
            println!("Moved block '{slug}'.");
            Ok(())
        }
    }
}

fn file(action: FileAction) -> Result<()> {
    match action {
        FileAction::Add {
            slug,
            path,
            note,
            after,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            require_in_scope(&ctx.source, &path)?;
            ctx.writer()
                .edit(|map, _| map.add_file(&slug, &path, note, after.as_deref()))?;
            println!("Added '{path}' to block '{slug}'.");
            Ok(())
        }
        FileAction::Update {
            slug,
            path,
            note,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            ctx.writer()
                .edit(|map, _| map.update_file(&slug, &path, Some(note)))?;
            println!("Updated the note on '{path}'.");
            Ok(())
        }
        FileAction::Remove { slug, path, scope } => {
            let ctx = open(&scope, None, None)?;
            ctx.writer().edit(|map, _| map.remove_file(&slug, &path))?;
            println!("Removed '{path}' from block '{slug}'.");
            Ok(())
        }
    }
}

fn line(action: LineAction) -> Result<()> {
    match action {
        LineAction::Add {
            slug,
            path,
            range,
            note,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            let (from, to) = parse_range(&range)?;
            require_in_scope(&ctx.source, &path)?;
            require_range_in_file(&ctx.source, &path, from, to)?;
            ctx.writer()
                .edit(|map, _| map.add_line_note(&slug, &path, from, to, note))?;
            println!("Added a note on {path}:{from}-{to}.");
            Ok(())
        }
        LineAction::Update {
            slug,
            path,
            range,
            note,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            let (from, to) = parse_range(&range)?;
            ctx.writer()
                .edit(|map, _| map.update_line_note(&slug, &path, from, to, note))?;
            println!("Updated the note on {path}:{from}-{to}.");
            Ok(())
        }
        LineAction::Remove {
            slug,
            path,
            range,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            let (from, to) = parse_range(&range)?;
            ctx.writer()
                .edit(|map, _| map.remove_line_note(&slug, &path, from, to))?;
            println!("Removed the note on {path}:{from}-{to}.");
            Ok(())
        }
        LineAction::Restore {
            slug,
            path,
            old_range,
            range,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            let (old_from, old_to) = parse_range(&old_range)?;
            let (from, to) = parse_range(&range)?;
            require_in_scope(&ctx.source, &path)?;
            require_range_in_file(&ctx.source, &path, from, to)?;
            ctx.writer().edit(|map, _| {
                let orphan = map.take_orphan(&slug, &path, old_from, old_to)?;
                map.add_line_note(&slug, &path, from, to, orphan.text)
            })?;
            println!("Restored the note at {path}:{from}-{to}.");
            Ok(())
        }
        LineAction::Discard {
            slug,
            path,
            old_range,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            let (from, to) = parse_range(&old_range)?;
            ctx.writer().edit(|map, _| {
                map.take_orphan(&slug, &path, from, to)?;
                Ok(())
            })?;
            println!("Discarded the note that was at {path}:{from}-{to}.");
            Ok(())
        }
    }
}

fn skim(action: SkimAction) -> Result<()> {
    match action {
        SkimAction::Add {
            path,
            reason,
            block,
            scope,
        } => {
            let ctx = open(&scope, None, None)?;
            require_in_scope(&ctx.source, &path)?;
            ctx.writer()
                .edit(|map, _| map.add_skim(&path, &reason, block))?;
            println!("Marked '{path}' as skim.");
            Ok(())
        }
        SkimAction::Remove { path, scope } => {
            let ctx = open(&scope, None, None)?;
            ctx.writer().edit(|map, _| map.remove_skim(&path))?;
            println!("'{path}' is no longer marked as skim.");
            Ok(())
        }
    }
}
