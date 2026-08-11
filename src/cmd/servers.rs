//! The reviews open on this machine.
//!
//! Not a review command: it answers about the machine, not about a repository,
//! so it never opens one. Standing anywhere is enough to ask what is running,
//! which is the point of asking.

use clap::{Args, Subcommand};

use crate::error::{Error, Result};
use crate::server::{Registry, ServerList};

#[derive(Args)]
pub(super) struct ServersArgs {
    #[command(subcommand)]
    action: Option<ServersAction>,
}

#[derive(Subcommand)]
enum ServersAction {
    /// List them. What you get when you name no action at all.
    List,
    /// Stop one, or all of them.
    Stop(StopArgs),
}

#[derive(Args)]
struct StopArgs {
    /// The port it is serving on, as `farol servers` prints it.
    port: Option<u16>,
    /// Every farol server on this machine.
    #[arg(long)]
    all: bool,
}

impl ServersArgs {
    pub(super) fn run(self) -> Result<()> {
        let registry = Registry::open()?;

        match self.action {
            None | Some(ServersAction::List) => {
                // Answering, not registered: the list is worth nothing if it
                // reports a review that is not there any more.
                print!("{}", ServerList(&registry.answering()?));
                Ok(())
            }
            Some(ServersAction::Stop(stop)) => stop.run(&registry),
        }
    }
}

impl StopArgs {
    fn run(self, registry: &Registry) -> Result<()> {
        let running = registry.running()?;

        match (self.port, self.all) {
            (Some(_), true) => Err(Error::msg("stop a port, or stop --all, not both")),

            (Some(port), false) => {
                let entry = running
                    .into_iter()
                    .find(|e| e.port == port)
                    .ok_or_else(|| {
                        Error::msg(format!(
                            "no farol server is running on port {port} — `farol servers` lists them"
                        ))
                    })?;
                registry.stop(&entry)?;
                println!("Stopped the server on port {port}.");
                Ok(())
            }

            (None, true) => {
                if running.is_empty() {
                    println!("No farol server is running.");
                    return Ok(());
                }
                // Every one of them, even if one refuses: leaving the rest up
                // because the first would not go is the opposite of --all.
                let mut refused = Vec::new();
                for entry in &running {
                    match registry.stop(entry) {
                        Ok(()) => println!("Stopped the server on port {}.", entry.port),
                        Err(e) => refused.push(e.to_string()),
                    }
                }
                match refused.is_empty() {
                    true => Ok(()),
                    false => Err(Error::msg(refused.join("\n"))),
                }
            }

            (None, false) => Err(Error::msg(
                "name the port to stop, or pass --all — `farol servers` lists them",
            )),
        }
    }
}
