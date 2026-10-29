//! What a thread is about.

use crate::diff::domain::Side;
use crate::lenses::domain::{LensId, LensTarget};
use crate::map::domain::Slug;

/// What a thread is about: a span of a file's lines, a lens, or the
/// description of a block.
///
/// One type the thread holds, rather than one kind of thread per thing on
/// screen: the conversation, the agent's answer and closing are the same
/// whatever it is about, and only where it sits differs. Every place that asks
/// where a thread sits asks this, and the match is exhaustive, so a kind added
/// later cannot be forgotten by one of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    Lines(Lines),
    Lens { lens: LensId, target: LensTarget },
    Block { slug: Slug },
}

impl Subject {
    /// The span of a file, when the thread is about one. Only such a thread
    /// can become a comment for the pull request: GitHub places a review
    /// comment on a line, and a lens or a block has none.
    pub fn lines(&self) -> Option<&Lines> {
        match self {
            Self::Lines(lines) => Some(lines),
            Self::Lens { .. } | Self::Block { .. } => None,
        }
    }

    /// Where it sits, the way a person would say it.
    pub fn at(&self) -> String {
        match self {
            Self::Lines(lines) => lines.at(),
            Self::Lens { lens, target } => format!("the {lens} lens on {target}"),
            Self::Block { slug } => format!("the description of block '{slug}'"),
        }
    }

    /// The command that shows what the thread is about, for an agent that
    /// cannot see the screen. A span of a file is read from the file itself.
    pub fn shown_by(&self) -> Option<String> {
        match self {
            Self::Lines(_) => None,
            Self::Lens { lens, target } => Some(match target {
                LensTarget::Pr => format!("farol lens show {lens}"),
                LensTarget::Block { slug } => format!("farol lens show {lens} --block {slug}"),
                LensTarget::File { path } => format!("farol lens show {lens} --file {path}"),
            }),
            Self::Block { .. } => Some("farol map show".into()),
        }
    }
}

/// A span of lines on one side of a file's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub path: String,
    /// Which side of the diff `from` and `to` are counted on.
    ///
    /// Not a detail of how it is displayed: the same pair of numbers names one
    /// passage on the old side and a different one on the new, and the host is
    /// told the side along with the numbers. A comment without it is a comment
    /// that could land on either.
    pub side: Side,
    pub from: u32,
    pub to: u32,
}

impl Lines {
    /// Where it sits, the way a person would type it to go there.
    ///
    /// The side is written only when it is the old one. Both sides number from
    /// one, so `src/a.rs:12` alone would be two different places — and the
    /// unmarked form has to keep meaning what it has always meant, which is the
    /// file as it now reads.
    pub fn at(&self) -> String {
        let lines = match self.from == self.to {
            true => format!("{}", self.from),
            false => format!("{}-{}", self.from, self.to),
        };
        match self.side {
            Side::New => format!("{}:{lines}", self.path),
            Side::Old => format!("{}:{lines} (old)", self.path),
        }
    }
}
