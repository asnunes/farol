//! Serving the review in a browser.

use clap::Args;

use super::{Ctx, ScopeFlags};
use crate::shared::error::Result;

#[derive(Args)]
pub(super) struct ServeArgs {
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
    pub(super) fn run(self) -> Result<()> {
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
            port: self.port,
            open_browser: !self.no_open,
            watch: !self.no_watch,
            git_dir: ctx.git_dir().clone(),
        })
        .run()
    }
}
