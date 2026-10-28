//! Lenses laid out for the terminal — mostly for the session that writes them.

use std::fmt::{self, Display};

use crate::lenses::application::{Pending, Why};
use crate::lenses::domain::{LensAnswer, LensCatalog, LensGroup, LensTarget};

/// The work the session owes, with everything it needs to do it: the prompt,
/// the rules farol holds every answer to, and the command for each target.
pub struct PendingReport<'a>(pub &'a [Pending]);

impl Display for PendingReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return writeln!(f, "Every lens is written and up to date.");
        }
        writeln!(f, "{}", RULES)?;
        let mut last = None;
        for pending in self.0 {
            if last != Some(&pending.lens.id) {
                writeln!(
                    f,
                    "\n{}  \"{}\"  ({})",
                    pending.lens.id,
                    pending.lens.title,
                    written(pending.lens.group)
                )?;
                writeln!(f, "  asks: {}", pending.lens.prompt)?;
                last = Some(&pending.lens.id);
            }
            let (id, at) = (&pending.lens.id, flag(&pending.target));
            match pending.why {
                Why::Missing => writeln!(f, "  not written: farol lens write {id}{at} --text '…'")?,
                Why::Outdated => writeln!(
                    f,
                    "  outdated: if it still holds, farol lens keep {id}{at}; if not, farol lens write {id}{at} --text '…'"
                )?,
            }
        }
        Ok(())
    }
}

/// One line for `map derive` and `map check`: updating the map is when the
/// lenses are reviewed, so the session is told there is work left.
pub struct LensReminder<'a>(pub &'a [Pending]);

impl Display for LensReminder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.len() {
            0 => Ok(()),
            1 => writeln!(f, "1 lens to review — run `farol lens pending`."),
            n => writeln!(f, "{n} lenses to review — run `farol lens pending`."),
        }
    }
}

/// The catalog, grouped as Settings shows it.
pub struct CatalogReport<'a>(pub &'a LensCatalog);

impl Display for CatalogReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for group in [LensGroup::Pr, LensGroup::Block, LensGroup::File] {
            writeln!(f, "{}:", heading(group))?;
            for entry in self
                .0
                .all()
                .into_iter()
                .filter(|e| e.definition.group == group)
            {
                let state = if entry.enabled { "on " } else { "off" };
                writeln!(
                    f,
                    "  [{state}] {}  \"{}\"",
                    entry.definition.id, entry.definition.title
                )?;
                writeln!(f, "        {}", entry.definition.prompt)?;
            }
        }
        Ok(())
    }
}

/// One answer, as written.
pub struct AnswerReport<'a>(pub &'a LensAnswer);

impl Display for AnswerReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let answer = self.0;
        writeln!(
            f,
            "{} on {}, written at {}",
            answer.lens,
            answer.target,
            crate::shared::short(&answer.written_at)
        )?;
        if answer.is_empty() {
            return writeln!(f, "(empty: the lens does not apply here)");
        }
        writeln!(f, "\n{}", answer.text.trim())?;
        for path in &answer.read {
            writeln!(f, "read yourself: {path}")?;
        }
        Ok(())
    }
}

const RULES: &str = "For an outdated lens, first check whether what it says still holds for the code as it is now: keep it if it does, rewrite it if not. Write each from what you know of the change, in the language of the map, in a few lines. Cite code in backticks as `path:from-to`, adding (old) for a removed line; a line that does not exist is refused. For a file lens, write an empty text for a file it does not apply to. Writing a lens changes no code: do not change, commit or publish anything while doing it.";

fn written(group: LensGroup) -> &'static str {
    match group {
        LensGroup::Pr => "once for the pull request",
        LensGroup::Block => "once per block",
        LensGroup::File => "once per file it applies to",
    }
}

fn heading(group: LensGroup) -> &'static str {
    match group {
        LensGroup::Pr => "PR",
        LensGroup::Block => "Block",
        LensGroup::File => "File",
    }
}

fn flag(target: &LensTarget) -> String {
    match target {
        LensTarget::Pr => String::new(),
        LensTarget::Block { slug } => format!(" --block {slug}"),
        LensTarget::File { path } => format!(" --file {path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_listed_by_group_with_what_is_on() {
        let out = CatalogReport(&LensCatalog::default()).to_string();

        assert!(out.starts_with("PR:"), "{out}");
        assert!(out.contains("[on ] overview"), "{out}");
        assert!(out.contains("[off] vocabulary"), "{out}");
        assert!(out.contains("File:\n  [on ] tests"), "{out}");
    }
}
