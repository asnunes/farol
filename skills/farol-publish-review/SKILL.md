---
name: farol-publish-review
description: Submit an existing Farol review — the pending comments already on the pull request, with a verdict — or synchronize its file-read marks, through the authenticated GitHub CLI account, after the reader approves the destination and action.
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
review mode, the host, the pending comments on the PR, and the current
file-read marks. Verify the repository and destination
PR with `gh pr view` in that repository (or with an explicit `--repo`). Stop on
missing prerequisites or a mismatch; do not switch revisions to force publication.

Farol obtains credentials for that host from `gh`; it does not configure or
store a separate token. Local reading and threads do not require `gh`.

## Gather what the reader already wrote

The comments that go are the pending comments `farol github status` lists:
they are already in the reader's pending review on the PR, written from the
review page or with `farol pending write`. Threads (`farol thread list`) stay
in Farol and are never sent. Use the existing read marks reported by
`farol github status`; never infer that an unmarked file has been read.

Do not find new defects, rewrite pending comments, or publish the map's
explanations or the threads as review comments.

## Ask which action to take

If the authenticated account is the PR author, explicitly say that this is
their own PR and ask them to choose:

- Submit their pending comments and synchronize file-read marks, using the
  comment verdict. Do not offer approval or request-changes on their own PR.
- Synchronize only file-read marks, leaving the pending comments pending.

For another author's PR, ask whether to publish a comment, approval, or request
for changes. Obtain the reader's summary when needed; do not invent a verdict
or summary. If they request only file-read synchronization, use that action.

## Confirm the concrete result

Present the full destination PR URL, the pending comments with their
locations, file-read marks, and the chosen summary/verdict or the marks-only
action. Obtain approval for that content and action. Reuse explicit
approval already given for the same content and destination.

Creating a map or conducting a local review does not authorize publication.

## Execute through Farol

For a verdict, use `farol github review` with exactly one of `--comment`,
`--approve`, or `--request-changes`, and the supplied `--summary` when
applicable. It submits the pending review as it is on the PR. Farol also
synchronizes current file-read marks when the PR shows the reviewed commit.

For marks only, run `farol github ticks` with the same comparison options.
This leaves the pending comments pending.

If Farol refuses, report the reason; do not alter the reviewed revision or
bypass it by posting or submitting directly with `gh`.

## Report what happened

Show the full published review URL as plain text, the number of comments sent,
and the number of files marked as read. For marks only, show the PR URL and
number of marks synchronized, and say that the pending comments are still
pending.

If the review was published but marking files failed, state both outcomes.
Do not publish the review again to retry its marks. If any result is unclear,
inspect its state before attempting another send.
