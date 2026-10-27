//! The lenses farol ships.
//!
//! Four are on, one for each thing "Solving code review at Anthropic" says
//! stays with a person once machines do most of the reading: deciding whether
//! the change should exist, reviewing the risky code and the patterns others
//! will copy, and reading the evidence. Tests is the list of cases the user
//! asked for in place of reading the test files. Each prompt asks for a few
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
        prompt: "What this pull request makes the code do that it did not do before, and why. Then the decisions that shaped it: what was chosen, the alternative that was dropped, and what the choice costs. A few short paragraphs; a reader who stops here should be able to explain the change.",
        enabled: true,
    },
    BuiltIn {
        id: "risk",
        title: "Risk",
        group: LensGroup::Pr,
        prompt: "How risky this change is, judged by what it touches rather than by who wrote it: low, medium or high, and why. Name the sensitive paths it touches (authentication, billing, data, CI, release), any pattern it introduces that others will copy, and the files a person should read themselves, passing each of those with --read. A few lines.",
        enabled: true,
    },
    BuiltIn {
        id: "evidence",
        title: "Evidence",
        group: LensGroup::Pr,
        prompt: "What was checked, and what it would miss. The commands that ran and what they returned; then the problems a fresh reviewer finds in the diff, each tagged Important, Nit or Pre-existing, with its file and line, including anything that breaks CLAUDE.md, AGENTS.md or REVIEW.md; nothing about style alone. Review the diff in a subagent with a clean context for this part. Say \"Nothing to fix\" when there is nothing. End with what these checks would not catch.",
        enabled: true,
    },
    BuiltIn {
        id: "tests",
        title: "Tests",
        group: LensGroup::File,
        prompt: "Write this for each file in the review that has tests, including a source file with tests inside it. List the tests this change adds or changes in that file, one sentence of behaviour each, with its line, so the reviewer learns the cases without reading the test code. Write an empty text for a file without tests.",
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
