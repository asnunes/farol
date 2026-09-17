---
name: farol-publish-review
description: Publish an existing Farol review or synchronize its file-read marks through the authenticated GitHub CLI account, after the reader approves the destination and action.
---

# Publish an existing review

## Prerequisites and destination

Read [farol](../farol/SKILL.md) first. If it is missing, guide the user to
install the complete Farol set.

This workflow requires GitHub CLI (`gh`). Check `gh --version`, identify the
repository's GitHub host, and run `gh auth status --hostname HOST` for that host.
If authentication is missing, guide the user through `gh auth login --hostname
HOST` in their terminal. Never request a token in the conversation.

Run `farol github status` with the intended comparison options. It reports the
PR number, its commit, whether it belongs to the authenticated account, the
host, and the current file-read marks. Verify the repository and destination
PR with `gh pr view` in that repository (or with an explicit `--repo`). Stop on
missing prerequisites or a mismatch; do not switch revisions to force publication.

Farol obtains credentials for that host from `gh`; it does not configure or
store a separate token. Local reading and commenting do not require `gh`.

## Gather what the reader already wrote

Use `farol comment list` to collect pending comments, including their old/new
side and line ranges. Preserve their text and any severity the reader supplied.
Use the existing read marks reported by `farol github status`; never infer that
an unmarked file has been read.

Do not find new defects, rewrite findings, or publish the map's explanations
as review comments.

## Ask which action to take

If the authenticated account is the PR author, explicitly say that this is
their own PR and ask them to choose:

- Publish their comments and synchronize file-read marks, using the comment
  verdict. Do not offer approval or request-changes on their own PR.
- Synchronize only file-read marks, leaving local comments unpublished.

For another author's PR, ask whether to publish a comment, approval, or request
for changes. Obtain the reader's summary when needed; do not invent a verdict
or summary. If they request only file-read synchronization, use that action.

## Confirm the concrete result

Present the full destination PR URL, pending comments with their severity and
line/side locations, file-read marks, and the chosen summary/verdict or the
marks-only action. Obtain approval for that content and action. Reuse explicit
approval already given for the same content and destination.

Creating a map or conducting a local review does not authorize publication.

## Execute through Farol

For comments and a verdict, use `farol github review` with exactly one of
`--comment`, `--approve`, or `--request-changes`, and the supplied `--summary`
when applicable. Farol also synchronizes current file-read marks.

For marks only, run `farol github ticks` with the same comparison options.
This does not publish or close local comments.

Farol validates the PR commit and comment paths, sides and ranges. If it refuses,
report the reason; do not relocate or drop comments, alter the reviewed revision,
or bypass validation by posting them directly with `gh`.

## Report what happened

Show the full published review URL as plain text, the number of comments sent,
and the number of files marked as read. For marks only, show the PR URL and
number of marks synchronized, and say that the comments remain local.

If the review was published but marking files failed, state both outcomes.
Do not publish the review again to retry its marks. If any result is unclear,
inspect its state before attempting another send.
