//! Serving the review in a browser.

use clap::Args;

use super::{Ctx, ScopeFlags, server_factory};
use crate::error::Result;
use crate::server::{Port, Registry, SessionConfig, detach};
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
    /// 4600 up; `0` asks the operating system for any. An existing instance
    /// always keeps its port.
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
        let workspace = Workspace::here()?;
        let root = workspace.root().canonicalize()?;
        let registry = Registry::open()?;
        if let Some(running) = registry.serving(&root)? {
            let entry = running.configure(&self.session())?;
            println!("farol updated the review at {}", entry.url());
            if self.port.is_some() {
                println!("Reused the existing port {}.", entry.port);
            }
            if !self.no_open {
                let _ = std::process::Command::new("open").arg(entry.url()).spawn();
            }
            return Ok(());
        }
        if self.foreground {
            return self.serve();
        }

        let entry = detach::spawn(&registry)?;
        println!("farol is reading at {}", entry.url());
        println!("stop it with: farol servers stop {}", entry.port);
        Ok(())
    }

    fn session(&self) -> SessionConfig {
        SessionConfig {
            scope: self
                .scope
                .with_base(self.base.clone())
                .request(self.head.clone()),
            watch: !self.no_watch,
        }
    }

    fn serve(self) -> Result<()> {
        let session = self.session();
        let workspace = Workspace::here()?;
        let root = workspace.root().canonicalize()?;
        let ctx = Ctx::at(&root, session.scope.clone())?;

        // Without a map there is nothing farol can show. Falling back to a plain
        // diff viewer would make it a worse version of tools that already do
        // that well, so it refuses in the terminal instead of opening a browser
        // onto an apology.
        ctx.show_map.require()?;
        let scope = ctx.scope.execute()?;

        crate::server::Server::new(crate::server::ServeConfig {
            factory: server_factory(root.clone()),
            check_head: super::wiring::check_head(&root)?,
            session,
            branch: scope.branch,
            base: scope.base_ref,
            port: self.port.map_or(Port::Free, |p| match p {
                0 => Port::Ephemeral,
                p => Port::Exactly(p),
            }),
            open_browser: !self.no_open,
            git_dir: ctx.git_dir().clone(),
            common_dir: workspace.common_dir().to_path_buf(),
            repo: root,
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
    fn a_new_invocation_carries_its_own_scope_and_watch_preference() {
        let config = args(&["develop", "HEAD", "--direct", "--no-watch"]).session();
        assert_eq!(config.scope.base.as_deref(), Some("develop"));
        assert_eq!(config.scope.head.as_deref(), Some("HEAD"));
        assert!(config.scope.direct);
        assert!(!config.watch);
        assert!(args(&[]).session().watch);
    }
}
