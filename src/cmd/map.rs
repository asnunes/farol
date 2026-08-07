//! Creating, inspecting and verifying the map itself.

use clap::Subcommand;

use super::{Action, Ctx};
use crate::map::application::ResetOutcome;
use crate::map::presentation::{CheckSummary, MapReport, OrphanReport};
use crate::shared::error::Result;

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
}

impl Action for MapAction {
    fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            MapAction::Derive => {
                let derived = ctx.derive_map.execute()?;
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
                        behind: ctx.derive_map.behind(&derived.map),
                        map: &derived.map,
                    }
                );
                print!("{}", OrphanReport(derived.map.orphans()));
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
