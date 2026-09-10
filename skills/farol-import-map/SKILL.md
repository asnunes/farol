---
name: farol-import-map
description: Obtain an author-provided Farol JSON map from a supplied path or URL, or from the indicated PR, and import it for the reviewer.
---

# Import the author's map

## Prerequisites

This workflow requires [farol](../farol/SKILL.md) and
[farol-pr-context](../farol-pr-context/SKILL.md).

Locate and read their instructions before proceeding. Identify missing
prerequisites and guide the user to install the complete Farol set.

## Obtain the map

If the user provides a local file path or a download URL, obtain the
JSON from that location.

Otherwise, use the indicated pull request to locate its attached map,
using the context gathered by `farol-pr-context`.

If multiple maps are available and the intended one is unclear, ask
which map to use.

If the map cannot be found or accessed, explain the problem and ask
the reviewer to provide the JSON or its local path. Use the appropriate
authenticated access for a private attachment; never send a GitHub token
to an arbitrary download host.

Continue with import only after obtaining the file on the reviewer's machine.

## Prepare the review checkout

Use `farol-pr-context` to identify the repository and comparison and prepare
the checkout safely. Reuse context already gathered for this import.

If the user supplied a map without identifying the PR, establish which
repository and change they intend to review before importing.

Preserve existing local work, following the checkout rules in
`farol-pr-context`.

## Import the map

Run `farol map import <file>` using the obtained JSON and the intended
comparison options.

Use the CLI to validate and import the file. Do not edit the JSON to
make it pass validation.

Preserve the author's explanations. Importing a map does not authorize
reconstructing it, rewriting its notes, or judging the implementation.

If Farol rejects the import, report the reason and resolve the file
or comparison mismatch before continuing.

If Farol reports that the map and checkout refer to different commits,
explain that difference to the reviewer. Do not silently present the
map as describing the exact current revision.

## Open the review

Inspect the imported map and follow the `farol` skill's server-handling
instructions to open the intended review.

Report the URL printed by `farol serve` and any relevant limitation
reported during import.

The workflow ends with the imported map available for the reviewer.
It does not publish comments or a review to GitHub.
