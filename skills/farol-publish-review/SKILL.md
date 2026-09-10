---
name: farol-publish-review
description: Gather an existing Farol review and publish the reviewer-approved comments, file-read marks, summary, and verdict to GitHub.
---

# Publish an existing review

## Prerequisites

This workflow requires [farol](../farol/SKILL.md). Read its instructions before
proceeding. If it is missing, guide the user to install the complete Farol set.

## Gather the review

Collect the pending reviewer comments and the existing file-read marks.

Preserve the comments and any severity already assigned by the reviewer.
Do not search for new defects, assign severity, rewrite findings, or
infer that an unmarked file has been read.

Keep map explanations separate from reviewer comments. The map is not
material to publish as review findings.

## Complete the publication choices

Use the summary and verdict supplied by the reviewer. Ask for missing
choices needed to publish; do not invent them.

If the reviewer is the PR author, only the comment verdict is available.
Explain that limitation when necessary, preserving the reviewer's
comments and their assigned severity.

For another person's PR, use the reviewer-selected comment, approval,
or request-changes verdict.

## Check the destination

Run `farol github status` with the intended comparison options.

Confirm that the destination is the pull request the reviewer intends
to publish to. If Farol reports a missing prerequisite or a mismatch,
explain it before proceeding.

If authentication is needed, direct the user to the Farol interface.
Do not ask them to paste a token into the conversation.

## Confirm the publication

Present the destination PR, pending comments with their existing
severity, file-read marks, and the supplied summary and verdict.

Obtain authorization for that content and action before sending.
Reuse explicit authorization already given for the same content
and destination.

Approval to create a map, import one, or conduct a local review does
not authorize publication to GitHub.

## Publish through Farol

Use `farol github review` with the reviewer-selected verdict and
the supplied summary.

Let Farol validate the current PR revision and comment locations.
If it refuses publication, report the reason. Do not silently change
the reviewed revision, relocate comments, or remove comments to make
the request succeed.

## Report the result

Display the full URL of the published review as plain text.
Report how many comments were sent and how many files were marked
as read.

If the review was published but file-read synchronization failed,
state both outcomes clearly. Do not send the review again merely
because its reading marks were not synchronized.

If publication fails or its outcome is unclear, inspect the available
result before attempting another send. Do not claim success without
confirmation.
