//! Serving the review in a browser.

use clap::Args;

use super::{Ctx, ScopeFlags};
use crate::error::Result;
use crate::server::{Port, Registry, detach};
use crate::shared::paths::Workspace;

#[derive(Args)]
pub(super) struct ServeArgs {
    /// Base ref. Defaults to main, then master.
    base: Option<String>,
    /// Head ref. Defaults to the branch you are on.
    head: Option<String>,
    #[command(flatten)]
    scope: ScopeFlags,
    /// Port to listen on. Omit it and farol takes the first free one from
    /// 4600 up; `0` asks the operating system for any.
    #[arg(long)]
    port: Option<u16>,
    /// Do not open a browser.
    #[arg(long)]
    no_open: bool,
    /// Do not watch the repository for changes.
    #[arg(long)]
    no_watch: bool,
    /// Hold the terminal instead of going into the background. What the
    /// detached server runs as, and how to watch one that will not start.
    #[arg(long)]
    foreground: bool,
}

impl ServeArgs {
    pub(super) fn run(self) -> Result<()> {
        if self.foreground {
            return self.serve();
        }

        let registry = Registry::open()?;

        // Asking twice for the same review should hand back the one already
        // open, not a second server on a second port. Only for a bare `farol
        // serve`: any flag means a different window was asked for, and a window
        // this one may not be showing.
        if self.is_plain() {
            let workspace = Workspace::here()?;
            if let Some(running) = registry.serving(&workspace.root(), workspace.branch())? {
                println!("farol is already reading at {}", running.url());
                return Ok(());
            }
        }

        let entry = detach::spawn(&registry)?;
        println!("farol is reading at {}", entry.url());
        println!("stop it with: farol servers stop {}", entry.port);
        Ok(())
    }

    /// Nothing in particular was asked for, so whatever is already running for
    /// this branch is the same review.
    fn is_plain(&self) -> bool {
        self.base.is_none() && self.head.is_none() && self.port.is_none() && self.scope.is_default()
    }

    fn serve(self) -> Result<()> {
        let scope = self.scope.with_base(self.base.clone());
        let ctx = Ctx::from_workspace(scope.request(self.head.clone()))?;

        // Without a map there is nothing farol can show. Falling back to a plain
        // diff viewer would make it a worse version of tools that already do
        // that well, so it refuses in the terminal instead of opening a browser
        // onto an apology.
        let map = ctx.show_map.require()?;

        crate::server::Server::new(crate::server::ServeConfig {
            use_cases: ctx.server().clone(),
            map,
            port: self.port.map_or(Port::Free, |p| match p {
                0 => Port::Ephemeral,
                p => Port::Exactly(p),
            }),
            open_browser: !self.no_open,
            watch: !self.no_watch,
            git_dir: ctx.git_dir().clone(),
            repo: ctx.root().clone(),
        })
        .run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// `serve` on its own, so these can ask about the arguments without going
    /// through the whole command surface.
    #[derive(Parser)]
    struct Serve {
        #[command(flatten)]
        args: ServeArgs,
    }

    fn args(extra: &[&str]) -> ServeArgs {
        Serve::try_parse_from(["farol"].into_iter().chain(extra.iter().copied()))
            .expect("serve should parse")
            .args
    }

    #[test]
    fn the_flag_the_detached_server_is_started_with_is_one_serve_accepts() {
        // `detach` appends this string to the command line. If the two ever
        // disagree, every detached start fails at the moment it is needed.
        assert!(args(&[detach::FOREGROUND]).foreground);
    }

    #[test]
    fn a_bare_serve_is_the_only_one_that_reuses_what_is_running() {
        assert!(args(&[]).is_plain());
        assert!(args(&["--no-open"]).is_plain(), "how the tests start one");

        // Each of these asks for a window the running server may not be showing.
        assert!(!args(&["develop"]).is_plain());
        assert!(!args(&["main", "feature/x"]).is_plain());
        assert!(!args(&["--port", "4700"]).is_plain());
        assert!(!args(&["--dirty"]).is_plain());
        assert!(!args(&["--direct"]).is_plain());
    }
}
