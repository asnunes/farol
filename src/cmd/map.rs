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
    /// Write the map to one file, to hand to whoever is going to review.
    Export {
        /// Where to write it. Defaults to map-<commit>.farol.json here.
        #[arg(long)]
        out: Option<String>,
    },
    /// Store a map somebody exported as a version here.
    Import { file: String },
}

impl Action for MapAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            MapAction::Export { out } => {
                let export = ctx.export_map.execute()?;
                let path = out.unwrap_or(export.file);
                std::fs::write(&path, &export.body).map_err(|source| {
                    crate::error::Error::CannotWrite {
                        path: path.clone(),
                        source,
                    }
                })?;

                println!("Wrote {path} ({} bytes).", export.body.len());
                println!(
                    "The map only. The code comes from git, and what you have read stays here."
                );
                Ok(())
            }
            MapAction::Import { file } => {
                let raw = std::fs::read_to_string(&file).map_err(|source| {
                    crate::error::Error::CannotRead {
                        path: file.clone(),
                        source,
                    }
                })?;
                let landed = ctx.import_map.execute(&raw)?;

                println!(
                    "Imported the map for {} on {}.",
                    short(&landed.map.generated_at),
                    landed.map.branch
                );
                // Said out loud rather than refused: a review is normally read
                // from a few commits ahead of the map that describes it.
                if let Some((written_at, here)) = landed.behind {
                    println!(
                        "It was written at {}, and you are on {} — the screen will say how far.",
                        short(&written_at),
                        short(&here)
                    );
                }
                print!("{}", MapSummary(&landed.map));
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
