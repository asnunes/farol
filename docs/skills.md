# Farol skills

Install the executable first using the [installation guide](install.md).
Skill installation uses Node/npm through `npx`; PR context and publication additionally require
Git, an authenticated GitHub CLI (`gh`), and Bash.

## Install the complete set

Run this inside the project you want to review:

```bash
npx skills add https://github.com/asnunes/farol/tree/main/skills --skill '*'
```

The source is the `skills/` directory, so the maintainer's `farol-release` skill
is not included. All seven user skills are installed together, including their
prerequisites. No automatic dependency resolution by the installer is assumed.

The command installs at project scope. Add `-g` for a user-wide installation,
or `--agent codex`, `--agent claude-code`, or another supported agent to choose
explicitly. Agent selection and installation locations are managed by
[npx skills](https://github.com/vercel-labs/skills).

In a Farol source checkout, the same set can be installed from a local path:

```bash
npx skills add ./skills --skill '*'
```

## Choose a workflow

| Skill | Use it for | Skill prerequisites |
|---|---|---|
| `farol` | Shared map principles, context, reading order, skim, and server handling | None |
| `farol-maintain-map` | Creating or updating a map in the implementation session | `farol` |
| `farol-pr-context` | Factual PR context inside a Farol workflow | Included checkout script; Git, gh, Bash |
| `farol-map-from-review` | Preparing a map when the reviewer received a PR without one | `farol`, `farol-pr-context`; delegates to import when a map exists |
| `farol-share-map` | Exporting the author's JSON for manual delivery | `farol`; maintenance if the map needs work |
| `farol-import-map` | Obtaining a JSON from a supplied path/URL or the PR and opening it | `farol`, `farol-pr-context` |
| `farol-publish-review` | Publishing the reviewer's existing comments, reading marks, summary, and verdict | `farol` |

Ask your agent to use a skill by name. In agents that support explicit skill
mentions, select it through their skill picker or invocation syntax.

Examples:

- “Use farol-maintain-map to explain the change we just implemented.”
- “Use farol-map-from-review for this PR, which has no map.”
- “Use farol-share-map and give me the full path of the exported JSON.”
- “Use farol-import-map with this PR and the JSON I received.”
- “Use farol-publish-review to prepare the comments I already wrote for approval.”

Map explanations give the reviewer context; they do not judge the code.
Publishing preserves the reviewer's comments and severity, and requires
approval of the content and destination. The author is told that it is their own PR and chooses between publishing
comments with read marks or synchronizing only read marks. Publishing on their
own PR uses the comment verdict. The skill checks authentication with `gh`;
the app has no publication or token-configuration UI.

Export and import are separate workflows on the author's and reviewer's
machines. Export gives the author the full absolute path of the JSON as plain
text. The author attaches or sends that file manually. Import uses the supplied
path or URL first; otherwise it looks for the map in the indicated PR and asks
for the file if it cannot obtain it.

## Shared instructions

Install the complete set in one scope so sibling references remain available.
Each workflow explicitly loads its prerequisite SKILL.md files; dependencies
are not assumed to load automatically. The PR checkout script travels with
`farol-pr-context` and does not depend on a maintainer's home directory.

The set adapts the existing local Farol and PR-context skills. It does not
rename or modify a separately installed personal `pr-context` skill.

This version installs no AI hooks or language-configuration mechanism. Those
remain separate decisions. Skills do not fix the output language to Portuguese;
provide the desired output language in the task when needed.

Installation support follows the agents supported by `npx skills`. Successful
file installation does not establish equivalent model behavior in every agent.
