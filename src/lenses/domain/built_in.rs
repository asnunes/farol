//! The lenses farol ships.
//!
//! Three are on for the pull request, one for each thing "Solving code review
//! at Anthropic" says stays with a person once machines do most of the
//! reading: deciding whether the change should exist, reviewing the risky code
//! and the patterns others will copy, and reading the evidence. Summary says
//! per block how its files relate, so the block is read as one piece rather
//! than file by file; Tests lists each file's cases; Smell is the one that
//! exists to open a file, and is empty on most. Each prompt asks for a few
//! lines, because lenses that take as long to read as the code have failed.

use super::{LensDefinition, LensGroup, LensId};

pub(super) struct BuiltIn {
    pub id: &'static str,
    pub title: &'static str,
    pub group: LensGroup,
    pub prompt: &'static str,
    pub enabled: bool,
    /// About how many lines an answer runs: shown to the session as a guide.
    pub about_lines: usize,
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
        prompt: "What this pull request makes the code do that it did not do before, and why. Then the decisions that shaped it: what was chosen, the alternative that was dropped, and what the choice costs. A reader who stops here should be able to explain the change to someone else.",
        enabled: true,
        about_lines: 6,
    },
    BuiltIn {
        id: "risk",
        title: "Risk",
        group: LensGroup::Pr,
        prompt: "How risky this change is, low, medium or high, judged by what it touches rather than by who wrote it, and why. Name the sensitive paths it touches, such as authentication, billing, data, CI or release, and any pattern it introduces that others will copy. Do not list what it leaves alone.",
        enabled: true,
        about_lines: 4,
    },
    BuiltIn {
        id: "evidence",
        title: "Evidence",
        group: LensGroup::Pr,
        prompt: "What was checked, and what it would miss. Say which commands ran and what they returned. Then only the problems that would change whether the reviewer approves, found by reviewing the diff in a subagent with a clean context, each marked Important or Pre-existing with its file and line, including anything that breaks CLAUDE.md, AGENTS.md or REVIEW.md. Give nits only as a count, and say there is nothing to fix when there is nothing. End with what these checks would not catch.",
        enabled: true,
        about_lines: 7,
    },
    BuiltIn {
        id: "summary",
        title: "Summary",
        group: LensGroup::Block,
        prompt: "What this block does and why, and how it is built, for a reader who has never seen its types. Start with what the block makes the code do and the decision behind it: what was chosen, the alternative dropped and what the choice costs. Then how its files fit together, starting from the file left open as the block's core and saying why the change lives there, then what the others do around it, each type or file named with what it does. Do not repeat the block's context, which is one sentence on the problem the block solves.",
        enabled: true,
        about_lines: 6,
    },
    BuiltIn {
        id: "tests",
        title: "Tests",
        group: LensGroup::File,
        prompt: "For a file with tests, the tests this change adds or changes, one per bullet, and each case of a table-driven or parametrized test counts as a test of its own. Say what each one checks in a plain sentence, the way a describe or it reads, never by the function's name, with its line cited. Write an empty text for a file without tests. With more tests than are worth reading, keep the ones a reviewer would ask about.",
        enabled: true,
        about_lines: 12,
    },
    BuiltIn {
        id: "smell",
        title: "Smell",
        group: LensGroup::File,
        prompt: "Only for a file whose code has a smell the reviewer should see before approving: duplicated logic, a responsibility in the wrong layer, a hidden coupling, an abstraction with one use, a rule broken from CLAUDE.md or AGENTS.md. Say what the smell is and where, with its line, and always pass --open '<the smell, in a few words>'. Write an empty text for every other file; style and naming are not smells.",
        enabled: true,
        about_lines: 3,
    },
    BuiltIn {
        id: "vocabulary",
        title: "Vocabulary",
        group: LensGroup::Pr,
        prompt: "The terms and types this pull request introduces or gives a new meaning to, each with what it is and where it lives. Only the ones a reader would stumble on.",
        enabled: false,
        about_lines: 6,
    },
    BuiltIn {
        id: "architecture",
        title: "Architecture",
        group: LensGroup::Block,
        prompt: "Where this block's files were put and why there, and what depends on what. Write an empty text for a block with nothing to say about it.",
        enabled: false,
        about_lines: 4,
    },
    BuiltIn {
        id: "security",
        title: "Security",
        group: LensGroup::Pr,
        prompt: "A security review of the diff: untrusted input, authentication, secrets, permissions. Each problem with its file and line; say there is nothing to fix when there is nothing.",
        enabled: false,
        about_lines: 5,
    },
];
