---
name: farol-share-map
description: Export an existing Farol map so the author can attach it to a pull request or send it directly to a reviewer.
---

# Share a review map

## Prerequisites

This workflow requires [farol](../farol/SKILL.md). Read it before proceeding.

If the map needs updating, use
[farol-maintain-map](../farol-maintain-map/SKILL.md) in the implementation
session before continuing. Identify missing skill dependencies instead
of proceeding without their instructions.

## Prepare the export

Identify the map and comparison the user intends to share.

Follow the general Farol verification workflow. If coverage or note
decisions remain unresolved, return to map maintenance.

The export must describe committed code that the reviewer can obtain.
If the map covers uncommitted work, explain what prevents the export;
do not create a commit merely to complete this workflow.

## Export the file

Run `farol map export` with the intended comparison options.

Use the filename and location reported by the command. Export through
the CLI rather than assembling or editing the JSON manually.

The generated JSON contains the review map: blocks, file assignments,
explanations, notes, and skim markers. It does not contain the source
code, reviewer comments, or marks indicating which files were read.

## Deliver the exported map

Display the full absolute path of the generated JSON as plain text.
Do not replace it with a labeled Markdown link.

Tell the author to attach that file to the PR or send it directly
to the reviewer.

This workflow ends when the exported file is delivered to the author.
Do not claim that the map was uploaded or sent.
