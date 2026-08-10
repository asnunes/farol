//! The files under review, as farol resolved them.

use super::{Action, Ctx};
use crate::error::Result;
use crate::map::presentation::ScopeReport;

/// `scope` takes no action of its own, but it still goes through the same door
/// as every other command: resolve the window, then run.
pub(super) struct ShowScope;

impl Action for ShowScope {
    fn run(self, ctx: &Ctx) -> Result<()> {
        print!("{}", ScopeReport(&ctx.scope.execute()?));
        Ok(())
    }
}
