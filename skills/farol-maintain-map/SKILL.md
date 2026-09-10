---
name: farol-maintain-map
description: Create and maintain a Farol map in the session implementing a change, recording session decisions and reconciling the map as the branch advances.
---

# Maintain the implementation map

Use this workflow in the session that implemented the change.

## Prerequisites

Read the [farol skill](../farol/SKILL.md) before proceeding. If it is missing,
identify the missing prerequisite and guide the user to install the complete
Farol skill set.

Apply its shared principles for self-contained explanations, reading order,
skim, map verification, and handling running reviews.

## Recover the implementation context

Before writing, revisit the conversation for decisions that shaped
the implementation: alternatives rejected, requirements clarified,
experiments, constraints, deliberate omissions, and corrections
made by the user.

Carry that context into the map as self-contained explanations.
Record what was chosen and why, leaving judgment to the reviewer.

## Establish the review scope

Check the CLI and the files under review:

    farol --version
    farol scope

If Farol is not installed, direct the user to its installation
instructions. Do not assume the user's project contains Farol's
build commands.

Use the intended base and comparison options consistently throughout
the workflow.

`farol scope` is the source of paths for the map. Use the paths it
reports rather than guessing filenames or copying them from an
outdated diff.

## Derive before editing

Start with:

    farol map derive
    farol map show

Derivation creates the map when necessary or returns the existing
version. When the branch has advanced, it can inherit and reconcile
the previous map and report notes that became inactive.

Read the existing map before changing it. Continue its explanation
and reading order where they still fit the implementation.

Do not regenerate the map from scratch merely because new commits
arrived. The reviewer may already be following its structure.

## Update the explanation

Compare the current implementation and session decisions with the
existing map.

- Add context for new decisions that affect the code.
- Revise explanations whose reasons or consequences changed.
- Add newly introduced files and place them in reading order.
- Revisit skim markers when a file gains a substantive change.
- Remove or revise explanations that no longer describe the code.

Preserve context that remains accurate. A new commit does not require
rewriting every block.

Use `farol file update` when changing a note on a file already assigned
to a block; `farol file add` is for adding the file.

## Reconcile notes when the branch moves

After committing on top of an existing map, run:

    farol map derive
    farol map show

A note whose code merely moved can shift with the changed line ranges.
A note whose passage was affected becomes inactive and requires a
decision: restore it where its explanation still applies, or discard it.

Inactive does not mean lost. Farol preserves the explanation and the
code snapshot that helps identify what it referred to.

## Locate the decision in the current implementation

Use the preserved snapshot to find the relevant code. Do not rely
only on the old line numbers: after a refactor, the same range may
contain something unrelated.

Read the current passage and revisit the session context:

- Does the explanation still describe the implementation?
- Does the recorded reason still apply?
- Did the decision move to another passage or file?
- Did a later decision replace it?

Resolve these questions from the code and the available session
evidence. Similar wording alone is not enough to restore a note.

## Restore or discard

If the explanation remains accurate, restore it to the corresponding
range:

    farol line restore <slug> <path> <old-range> --range <new-range>

If the explanation no longer applies, discard the inactive note:

    farol line discard <slug> <path> <old-range>

When the decision changed or moved somewhere the restore operation
does not cover, discard the inactive note and write an accurate
explanation at the appropriate block, file, or line.

Do not restore a note merely to make verification pass. If the reason
cannot be established, identify what is unknown rather than inventing
a justification.

## Preserve the existing review

Use derivation to continue the map. `farol map reset` deletes the newest
version and falls back to an earlier one; it is not the normal update
procedure.

Reading progress is tracked separately from the map. Files remain
marked as read while their content is unchanged; a content change
requires them to be read again.

If the user is already reviewing and the task requires another commit,
tell them before advancing the branch. Derive the map again and resolve
its inactive notes before handing the updated review back.

## Edit the map without losing context

Inspect the current map before choosing an editing command.

- Use `farol file add` to assign a file to a block.
- Use `farol file update` to change the note on an existing assignment.
- Use `farol --help` and the relevant command's `--help` when the syntax
  or supported operation is unclear.

Do not remove and recreate an existing assignment merely to update
its explanation. Preserve its place in the reading order unless
the implementation now requires a different position.

## Pass explanations as literal text

Shell syntax can change the text before Farol receives it.

Backticks inside double quotes execute commands. Use single quotes
when an explanation contains backticks, and handle embedded quotes
without changing the intended text.

After writing, inspect `farol map show` to confirm that the explanation
arrived intact. Command output accidentally inserted into a note is
not implementation context.

## Respect the current file and line range

Use paths reported by `farol scope` and inspect the current file before
adding a line note.

A deleted file can belong to a block or be marked as skim, but it
cannot receive a line note: there is no current passage to attach it to.
Explain the reason for its deletion at block or file level when that
reason helps the reviewer understand the change.

When Farol rejects a path, block, or range, read the error and inspect
the current state before correcting the command. Do not keep trying
guessed values.

## Keep map explanations separate from reviewer comments

Map notes explain the implementation and the decisions behind it.
Reviewer comments contain questions, findings, or requests arising
from the review.

Maintaining the map must not close, rewrite, or publish reviewer
comments unless the user also requested that work.

Writing a local map or comment does not authorize sending a review
to GitHub. Publication belongs to its own workflow.

## Verify and hand over

Finish with:

    farol map check
    farol map show

Resolve pending notes and coverage gaps, then reread the complete map
for accuracy, self-contained context, and reading order.

Before opening the review, inspect `farol servers`. Use `farol serve`
for the intended worktree and comparison, following the shared rules
for preserving running reviews.
