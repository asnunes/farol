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
                writeln!(
                    f,
                    "  limit: {} lines, refused past it",
                    pending.lens.max_lines()
                )?;
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
        if let Some(why) = &answer.open {
            writeln!(f, "opens the file: {why}")?;
        }
        writeln!(f, "\n{}", answer.text.trim())?;
        Ok(())
    }
}

const RULES: &str = "A lens exists to save the reader time. If reading it takes anywhere near as long as reading the code, it failed, and the reader you were writing for paid for it. Write the least that lets them decide: every lens has a hard limit of lines, and lines of at most 140 characters, which farol lens write refuses to pass; aim well under it, not at it. Point first, in bold; then one-line bullets, one item each, never a list chained with semicolons; no paragraphs. Cut whatever would not change whether they approve. For an outdated lens, first check whether what it says still holds for the code as it is now: keep it if it does, rewrite it if not; a keep is held to the same limits. Write in the language of the map. Cite code in backticks as `path:from-to`, adding (old) for a removed line; a line that does not exist is refused. For a file lens, write an empty text for a file it does not apply to; a file lens with more characters than the lines its change adds and removes is refused, because the code would be quicker to read. A file lens may open its file with --open '<why>', and that is the exception: only when reading the code could change whether the reviewer approves, saying what they will see. Writing a lens changes no code: do not change, commit or publish anything while doing it.";

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
