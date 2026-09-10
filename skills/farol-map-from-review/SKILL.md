---
name: farol-map-from-review
description: Prepare a Farol map for a pull request that has no author-provided map.
---

# Prepare a PR without a map

## Prerequisites

This workflow requires [farol](../farol/SKILL.md) and
[farol-pr-context](../farol-pr-context/SKILL.md).

Locate and read both before proceeding. If either is missing, identify
the missing prerequisite and guide the user to install the complete Farol set.

Follow `farol` for map construction, explanations, reading order, skim,
verification, and handling running reviews.

## Gather the PR context

Use `farol-pr-context` to identify the repository and exact comparison,
align the checkout safely, and gather the available factual context.

Preserve its source attribution and any reported uncertainty.
Do not proceed with map creation if the checkout does not represent
the intended review.

## Choose whether to build or import

Check whether a map is already available locally or associated with
the pull request.

If an author-provided map is available, continue through
[farol-import-map](../farol-import-map/SKILL.md). If a local map already exists,
inspect and preserve that work before deciding what needs to be added.

## Build the missing map

Use `farol scope` with the comparison established by `farol-pr-context`,
then run `farol map derive` with that same comparison.

Construct the map following the general `farol` guidance. Use the
context gathered by `farol-pr-context` without claiming access to decisions
that were not documented.

Finish with the general skill's map-verification and server-handling
workflow, then hand the map to the reviewer.
