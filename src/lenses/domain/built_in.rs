//! The lenses farol ships.
//!
//! Three are on for the pull request, one for each thing "Solving code review
//! at Anthropic" says stays with a person once machines do most of the
//! reading: deciding whether the change should exist, reviewing the risky code
//! and the patterns others will copy, and reading the evidence. Summary says
//! in a line or three what each file does, and Tests lists the cases, both so
//! a file can be judged without opening it; Smell is the one that exists to
//! open a file, and is empty on most. Each prompt asks for a few
//! lines, because lenses that take as long to read as the code have failed.

use super::{LensDefinition, LensGroup, LensId};

pub(super) struct BuiltIn {
    pub id: &'static str,
    pub title: &'static str,
    pub group: LensGroup,
    pub prompt: &'static str,
    pub enabled: bool,
}

impl BuiltIn {
    pub fn definition(&self) -> LensDefinition {
        LensDefinition {
            id: LensId::parse(self.id).expect("built-in ids are well formed"),
            title: self.title.into(),
            group: self.group,
            prompt: self.prompt.into(),
        }
    }
}

pub(super) fn built_in(id: &str) -> Option<&'static BuiltIn> {
    BUILT_IN.iter().find(|lens| lens.id == id)
}

pub(super) const BUILT_IN: &[BuiltIn] = &[
    BuiltIn {
        id: "overview",
        title: "Overview",
        group: LensGroup::Pr,
        prompt: "One bold line: what this pull request makes the code do that it did not do before, and why. Then the decisions that shaped it, at most four bullets of one line each: **the choice**, the alternative dropped, what it costs. No paragraphs; a reader who stops here should be able to explain the change.",
        enabled: true,
    },
    BuiltIn {
        id: "risk",
        title: "Risk",
        group: LensGroup::Pr,
        prompt: "Open with the level in bold, **Low**, **Medium** or **High**, and the reason, on one line. Then one bullet of one line for each sensitive path it touches (authentication, billing, data, CI, release) and each pattern it introduces that others will copy. Judge by what it touches, not by who wrote it, and do not list what it leaves alone.",
        enabled: true,
    },
    BuiltIn {
        id: "evidence",
        title: "Evidence",
        group: LensGroup::Pr,
        prompt: "Three short parts, each a bold label and one-line bullets. **Checked:** each command that ran and what it returned. **Problems:** only those that matter, found by reviewing the diff in a subagent with a clean context, each tagged Important or Pre-existing with its file and line, including anything that breaks CLAUDE.md, AGENTS.md or REVIEW.md; nits only as a count, and \"Nothing to fix\" when there is nothing. **Misses:** what these checks would not catch.",
        enabled: true,
    },
    BuiltIn {
        id: "summary",
        title: "Summary",
        group: LensGroup::File,
        prompt: "Write this for every file in the review whose change takes reading. In one to three short lines, say what the file does in this change, as plainly as: \"Defines the structs X and Y\", or \"Defines the use case Z, which: 1. … 2. … 3. …\". No more than it takes to know whether to open the file. Write an empty text when the diff is a few lines that say it by themselves: a summary is never longer than the change it summarizes. Pass --open '<why>' only when the code has a particularity these lines cannot carry and the reviewer should see it: an ordering, a boundary condition, a trick that works for a subtle reason. Name it in the why. Most files have none.",
        enabled: true,
    },
    BuiltIn {
        id: "tests",
        title: "Tests",
        group: LensGroup::File,
        prompt: "Write this for each file in the review that has tests, including a source file with tests inside it. One bullet per test this change adds or changes: what it checks, as a plain sentence the way a describe or it reads (\"routes by the offer's phase\"), never the test's function name, with its lines cited. Under it, the cases it covers as short nested bullets, one case each. Never chain cases with semicolons. Write an empty text for a file without tests.",
        enabled: true,
    },
    BuiltIn {
        id: "smell",
        title: "Smell",
        group: LensGroup::File,
        prompt: "Only for a file whose code has a smell the reviewer should see before approving: duplicated logic, a responsibility in the wrong layer, a hidden coupling, an abstraction with one use, a rule broken from CLAUDE.md or AGENTS.md. One line per smell, with its line, and always pass --open '<the smell, in a few words>'. Write an empty text for every other file; style and naming are not smells.",
        enabled: true,
    },
    BuiltIn {
        id: "vocabulary",
        title: "Vocabulary",
        group: LensGroup::Pr,
        prompt: "The terms and the types this pull request introduces or gives a new meaning to: one line each, what it is and where it lives.",
        enabled: false,
    },
    BuiltIn {
        id: "architecture",
        title: "Architecture",
        group: LensGroup::Block,
        prompt: "Where this block's files were put and why there, and what depends on what. A few lines; write an empty text for a block with nothing to say about it.",
        enabled: false,
    },
    BuiltIn {
        id: "security",
        title: "Security",
        group: LensGroup::Pr,
        prompt: "A security review of the diff: untrusted input, authentication, secrets, permissions. Each problem with its file and line; say \"Nothing to fix\" when there is nothing.",
        enabled: false,
    },
];
