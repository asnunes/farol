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
    /// Non-empty lines the answer may take; `farol lens write` refuses more.
    pub max_lines: usize,
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
        prompt: "What this pull request makes the code do that it did not do before, and why, in one bold line. Then at most three one-line bullets for the decisions that shaped it: **the choice**, the alternative dropped, what it costs. At most 5 lines; a reader who stops here can explain the change.",
        enabled: true,
        max_lines: 5,
    },
    BuiltIn {
        id: "risk",
        title: "Risk",
        group: LensGroup::Pr,
        prompt: "The level in bold, **Low**, **Medium** or **High**, and the reason, on one line. Then at most three one-line bullets: a sensitive path it touches (authentication, billing, data, CI, release) or a pattern others will copy. Judge by what it touches, not by who wrote it. Never list what it leaves alone. At most 4 lines.",
        enabled: true,
        max_lines: 4,
    },
    BuiltIn {
        id: "evidence",
        title: "Evidence",
        group: LensGroup::Pr,
        prompt: "**Checked:** the commands that ran and what they returned, one line. **Problems:** only those that would change whether the reviewer approves, found by reviewing the diff in a subagent with a clean context, each tagged Important or Pre-existing with its file and line, one line each; nits only as a count; \"Nothing to fix\" when there is nothing. **Misses:** one line. At most 7 lines.",
        enabled: true,
        max_lines: 7,
    },
    BuiltIn {
        id: "summary",
        title: "Summary",
        group: LensGroup::File,
        prompt: "What this file does in the change that its name does not already say, and how or why where the reader would not guess it. As long as the file needs and no longer, and always far quicker to read than its diff: farol refuses a file lens with more characters than the lines the change adds and removes. Never restate the name: \"Defines CodeBar: the strip where the block's code starts\" says nothing; \"Draws the strip between a block's lenses and its files, with the file count and the lines added and removed, so the reader knows how much code they would open\" says what, how and why. If someone holding only the file name could write it, rewrite it. Empty when the diff says it by itself, and then the file shows its code. Pass --open '<why>' only for a particularity these lines cannot carry: an ordering, a boundary, a trick that works for a subtle reason. Most files have none.",
        enabled: true,
        max_lines: 8,
    },
    BuiltIn {
        id: "tests",
        title: "Tests",
        group: LensGroup::File,
        prompt: "For a file with tests, one flat bullet per test this change adds or changes, and each case of a table-driven or parametrized test is a test of its own. Each bullet says what it checks in a few words, the way a describe or it reads, never the function name, with its line cited. No nesting, no grouping under the function. Empty for a file without tests. At most 15 lines: with more tests than fit, keep the ones a reviewer would ask about.",
        enabled: true,
        max_lines: 15,
    },
    BuiltIn {
        id: "smell",
        title: "Smell",
        group: LensGroup::File,
        prompt: "Only for a file whose code has a smell the reviewer should see before approving: duplicated logic, a responsibility in the wrong layer, a hidden coupling, an abstraction with one use, a rule broken from CLAUDE.md or AGENTS.md. One line per smell with its line, and always --open '<the smell, in a few words>'. Empty for every other file; style and naming are not smells. At most 3 lines.",
        enabled: true,
        max_lines: 3,
    },
    BuiltIn {
        id: "vocabulary",
        title: "Vocabulary",
        group: LensGroup::Pr,
        prompt: "The terms and types this pull request introduces or gives a new meaning to: one line each, what it is and where it lives. At most 6 lines; only the ones a reader would stumble on.",
        enabled: false,
        max_lines: 6,
    },
    BuiltIn {
        id: "architecture",
        title: "Architecture",
        group: LensGroup::Block,
        prompt: "Where this block's files were put and why there, and what depends on what, in one-line bullets. Empty for a block with nothing to say about it. At most 4 lines.",
        enabled: false,
        max_lines: 4,
    },
    BuiltIn {
        id: "security",
        title: "Security",
        group: LensGroup::Pr,
        prompt: "A security review of the diff: untrusted input, authentication, secrets, permissions. One line per problem with its file and line; \"Nothing to fix\" when there is nothing. At most 5 lines.",
        enabled: false,
        max_lines: 5,
    },
];
