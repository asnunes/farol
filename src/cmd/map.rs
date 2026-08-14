//! Creating, inspecting and verifying the map itself.

use clap::Subcommand;

use super::{Action, Ctx};
use crate::error::Result;
use crate::map::application::ResetOutcome;
use crate::map::presentation::{CheckSummary, MapReport, MapSummary, OrphanReport, short};

#[derive(Subcommand)]
pub(super) enum MapAction {
    /// Produce the map version for the current commit. Safe to call twice.
    Derive,
    /// Print the map.
    Show,
    /// Verify coverage and pending decisions. Non-zero exit when it fails.
    Check,
    /// Delete the newest version and fall back to the one before it.
    Reset,
    /// Write the map to one file, to hand to somebody else.
    Export {
        /// Where to write it. The name ends in .json so it can be attached to
        /// a pull request, which refuses extensions it does not know.
        #[arg(long, default_value = "review.farol.json")]
        out: String,
    },
    /// Take a map somebody exported and store it as a version here.
    Import {
        file: String,
        /// Replace a map already stored at that commit.
        #[arg(long)]
        over: bool,
    },
}

impl Action for MapAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            MapAction::Export { out } => {
                let bundle = ctx.share_map.export()?;
                std::fs::write(&out, &bundle)?;
                println!("Wrote {out} ({} bytes).", bundle.len());
                println!(
                    "It carries the map only — the code comes from git, and what you have read stays here."
                );
                Ok(())
            }
            MapAction::Import { file, over } => {
                let raw = std::fs::read_to_string(&file)
                    .map_err(|e| crate::error::Error::msg(format!("cannot read {file}: {e}")))?;
                let map = ctx.share_map.import(&raw, over)?;
                println!(
                    "Imported the map for {} on {}.",
                    short(&map.generated_at),
                    map.branch
                );
                print!("{}", MapSummary(&map));
                Ok(())
            }
            MapAction::Derive => {
                let derived = ctx.derive_map.execute()?;
                let map = &derived.map;
                let at = short(&map.generated_at);

                // Not the whole map: `map show` prints that, and after deriving
                // the reader wrote most of it themselves. What they cannot know
                // without being told is where this version came from and what
                // came loose on the way.
                match (derived.created, &map.parent) {
                    (true, Some(parent)) => {
                        println!(
                            "Created the map for {at}, inherited from {}.",
                            short(parent)
                        )
                    }
                    (true, None) => println!("Started the map for {at}. Nothing is mapped yet."),
                    (false, _) => println!("The map for {at} already exists — continuing from it."),
                }

                if !map.is_empty() {
                    print!("{}", MapSummary(map));
                }
                print!("{}", OrphanReport(map.orphans()));
                Ok(())
            }
            MapAction::Show => {
                match ctx.show_map.execute()? {
                    Some(map) => print!(
                        "{}",
                        MapReport {
                            behind: ctx.show_map.behind(&map),
                            map: &map,
                        }
                    ),
                    None => {
                        println!("No map for this branch yet. Run `farol map derive` to start one.")
                    }
                }
                Ok(())
            }
            MapAction::Check => {
                let report = ctx.check_map.execute()?;
                print!("{}", CheckSummary(&report));
                if report.passed() {
                    Ok(())
                } else {
                    std::process::exit(1)
                }
            }
            MapAction::Reset => {
                match ctx.reset_map.execute()? {
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
