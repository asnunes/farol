//! Lenses: what the session says about the change, so the reviewer can learn
//! it before the code.

use clap::Subcommand;

use super::{Ctx, ScopeArgs};
use crate::error::Result;
use crate::lenses::application::Aim;
use crate::lenses::domain::{LensGroup, LensId};
use crate::lenses::presentation::{AnswerReport, CatalogReport, PendingReport};
use crate::map::domain::Slug;

#[derive(Subcommand)]
pub(super) enum LensAction {
    /// Every lens, by group, with what each asks and whether it is on.
    List,
    /// Make a lens.
    Add {
        /// pr, block or file.
        #[arg(long)]
        group: String,
        #[arg(long)]
        title: String,
        /// What the session follows to write it. It can just name a skill.
        #[arg(long, allow_hyphen_values = true)]
        prompt: String,
    },
    /// Change a lens's title or prompt.
    Set {
        lens: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, allow_hyphen_values = true)]
        prompt: Option<String>,
    },
    Enable {
        lens: String,
    },
    Disable {
        lens: String,
    },
    /// Remove a lens for good, one farol ships included.
    Delete {
        lens: String,
    },
    /// What the session still has to write: missing or outdated answers.
    Pending,
    /// What a lens says about its target.
    Show {
        lens: String,
        #[command(flatten)]
        target: TargetArgs,
    },
    /// Keep what a lens said, after checking it still holds for the code as it is now.
    Keep {
        lens: String,
        #[command(flatten)]
        target: TargetArgs,
    },
    /// Write what a lens says about its target, replacing what it said.
    Write {
        lens: String,
        #[command(flatten)]
        target: TargetArgs,
        /// Markdown. Cite code in backticks as `path:from-to`. Empty means the
        /// lens does not apply here.
        #[arg(long, allow_hyphen_values = true)]
        text: String,
        /// A file a person should read themselves. Repeat for several.
        #[arg(long = "read")]
        read: Vec<String>,
    },
}

#[derive(clap::Args)]
pub(super) struct TargetArgs {
    /// The block, for a lens written per block.
    #[arg(long, conflicts_with = "file")]
    block: Option<String>,
    /// The file, for a lens written per file.
    #[arg(long)]
    file: Option<String>,
}

impl LensAction {
    /// The catalog belongs to the user, not to a repository, so those commands
    /// answer from anywhere; the answers need the review they are about.
    pub(super) fn run(self, scope: ScopeArgs) -> Result<()> {
        let catalog = || super::wiring::config_cases();
        let show = |c: crate::lenses::domain::LensCatalog| {
            print!("{}", CatalogReport(&c));
            Ok(())
        };
        match self {
            Self::List => show(catalog()?.list_lenses.execute()?),
            Self::Add {
                group,
                title,
                prompt,
            } => {
                let group = LensGroup::parse(&group)?;
                show(catalog()?.add_lens.execute(&title, group, &prompt)?)
            }
            Self::Set {
                lens,
                title,
                prompt,
            } => show(catalog()?.update_lens.execute(
                &LensId::parse(&lens)?,
                title.as_deref(),
                prompt.as_deref(),
            )?),
            Self::Enable { lens } => show(catalog()?.enable_lens.execute(&LensId::parse(&lens)?)?),
            Self::Disable { lens } => {
                show(catalog()?.disable_lens.execute(&LensId::parse(&lens)?)?)
            }
            Self::Delete { lens } => show(catalog()?.delete_lens.execute(&LensId::parse(&lens)?)?),
            Self::Pending => {
                let ctx = scope.open()?;
                print!("{}", PendingReport(&ctx.pending_answers.execute()?));
                Ok(())
            }
            Self::Show { lens, target } => {
                let ctx = scope.open()?;
                let aim = target.aim(&ctx)?;
                match ctx.show_answer.execute(&LensId::parse(&lens)?, aim)? {
                    Some(answer) => print!("{}", AnswerReport(&answer)),
                    None => println!("Nothing written for this lens here yet."),
                }
                Ok(())
            }
            Self::Keep { lens, target } => {
                let ctx = scope.open()?;
                let aim = target.aim(&ctx)?;
                let answer = ctx.keep_answer.execute(&LensId::parse(&lens)?, aim)?;
                println!("Kept lens '{}' on {}.", answer.lens, answer.target);
                Ok(())
            }
            Self::Write {
                lens,
                target,
                text,
                read,
            } => {
                let ctx = scope.open()?;
                let aim = target.aim(&ctx)?;
                let read = ctx.scope.paths(&read)?;
                let answer = ctx
                    .write_answer
                    .execute(&LensId::parse(&lens)?, aim, text, read)?;
                println!("Wrote lens '{}' on {}.", answer.lens, answer.target);
                Ok(())
            }
        }
    }
}

impl TargetArgs {
    fn aim(self, ctx: &Ctx) -> Result<Aim> {
        Ok(match (self.block, self.file) {
            (Some(block), _) => Aim::Block(Slug::parse(&block)?),
            (None, Some(file)) => Aim::File(ctx.scope.path(&file)?),
            (None, None) => Aim::Pr,
        })
    }
}
