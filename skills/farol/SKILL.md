---
name: farol
description: Apply Farol's principles for organizing review maps, writing implementation context, choosing reading order, and identifying files the reviewer can skip.
---

# Review maps with Farol

Farol shows a change in the order that makes its implementation understandable,
carrying the context the diff does not.

A map contains blocks, explanations, file and line notes, and skim markers.
Reviewer comments and reading progress are separate from the map.

Before using the CLI, check `farol --version`. If it is unavailable, direct the
user to the [installation guide](https://github.com/asnunes/farol/blob/main/docs/install.md).
Do not assume the user's project contains Farol's build commands. Use
`farol --help` and the relevant command's `--help` for command syntax.

## The principle

The map carries the context the diff does not.

When you implemented the change, its most valuable context comes from what
you lived in that session: the approach tried and reverted, the constraint
discovered along the way, the correction the user made, what is essential
and what is mechanical.

A map written by re-reading only the diff loses that context and becomes
a paraphrase of the code — which the reviewer already has.

Before writing, recover the context available to you. In the implementation
session, re-read the conversation for abandoned approaches, decisions,
constraints, corrections, and traps. As a reviewer, use the PR description,
discussion, documentation, and code as your sources.

With no implementation history, inferring from the code is legitimate as
long as it is marked as inference. Never present an inferred explanation
as a decision you witnessed.

The reader arrives cold. They did not implement this, may not have read
the issue, and usually did not write the pull request description. They
know the repository and nothing else about why this change exists.

Every block must give them enough context to understand its place in the
change, even when they open it directly from the sidebar.

## Form the blocks

A block is what you can describe in one sentence without needing the word
“and”. It is not a file, a commit, a directory, or a layer. A block cuts
across folders, and a file can appear in more than one.

Do not group by folder or by the file's role — controller, service,
repository. That grouping does not depend on the change: two completely
different changes would produce the same split.

Ask of every file whether it needs close reading before deciding which
block it belongs to. Mechanical changes may belong in skim, attached to
the block whose decision caused them.

Do not force a file into a thematic block just to avoid leftovers.
A misleading grouping is worse than acknowledging that a change stands
apart.

## Choose the reading order

To understand this, what does the reviewer need to have understood already?

Order by that dependency, both between blocks and inside each block.
On a tie, put the most central part first. There is no fixed template:
“contract, implementation, test” is sometimes the right order and
sometimes not.

The first block answers why this change exists. Without it, the rest
has nothing to stand on.

A test goes immediately after the file it tests. The reader has just
finished the implementation and wants to know whether its behavior is
covered; placing unrelated files between them makes the reader recover
that context again.

## Write the context the code cannot carry

The map's most valuable explanations connect a visible implementation
choice to a decision made during the session.

The code shows what was implemented. The session may explain why this
approach was chosen, which alternative was rejected, what constraint
shaped it, or why an apparent omission was deliberate.

Before writing, revisit the session for decisions that left a mark on
the implementation:

- An alternative considered and rejected.
- A requirement clarified or corrected by the user.
- A failure, experiment, or observation that changed the approach.
- A deliberate limit: behavior the implementation does not handle.
- A concrete choice whose reason is not apparent in the code, such as
  a fixed value, a concurrency strategy, or the absence of coordination.

Give these decisions priority in the map. Put them at block, file, or
line level according to where their consequences appear.

Connect three things:

1. The choice visible in the implementation.
2. The session evidence that explains it.
3. The consequence or boundary the reviewer should understand.

Evidence can be an explicit user decision, an observed result, or an
alternative discussed and rejected. Name that evidence concretely;
“this was discussed in the session” explains nothing.

Do not reconstruct a plausible justification and present it as a
remembered decision. If the session does not establish the reason,
say what is known and identify any inference.

This requirement does not remove orientation from the map. A sentence
explaining what the whole change enables belongs there, even when
someone could work it out from the diff.

## Make every explanation self-contained

The reviewer does not have access to the implementation session.
Everything needed to understand an explanation must be present in
the map itself.

A block context, file note, or line note must stand on its own.
The reviewer may encounter it directly, without reading earlier blocks
or the conversation that produced it.

When a session decision explains the implementation, carry the decision
and its relevant context into the note. Do not merely refer to them.

For example, if these facts were established in the session:

- Insufficient: “We kept the fixed limit as discussed.”
- Self-contained: “The limit is fixed at 30 seconds because the startup
  experiment on the larger repository exceeded the original limit.
  This bound applies to starting the local server, not to requests
  made while reading a review.”

Name the constraint, observation, or requirement that shaped the choice.
The session is the source of the explanation, not a reference the
reviewer must consult to understand it.

## Explain the implementation; leave judgment to the reviewer

The map equips the reviewer to judge the change. It does not perform
that judgment.

Explain what was implemented, why that approach was chosen, which
alternatives were considered, and what boundaries were deliberately
left in place.

Describe the reasons recorded in the session without endorsing them.
A decision being intentional does not establish that it was correct.

For example, if these facts were established in the session:

- Evaluative: “A lock would be unnecessary complexity here.”
- Explanatory: “The session separated map and progress into files owned
  by different writers and did not introduce a shared lock. Concurrent
  writes to the same map were not addressed in this change.”

Do not turn map notes into findings, recommendations, approvals,
or claims that the implementation is correct or sufficiently safe.

When a limitation matters, describe it factually. Whether that
limitation is acceptable, or requires a different implementation,
is the reviewer's decision.

If you identify a potential problem while preparing the map, keep
that finding separate from the map and report it through the review
or conversation workflow appropriate to the task.

## Give each block its context

A block context has three moves, in this order:

1. Where this block sits in the change. For the first block, explain the
   whole change in one sentence. For later blocks, reconnect the reader
   to it in a clause.
2. What this block does, with a verb.
3. The decision recovered from the session, the evidence explaining it,
   and its consequence in the implementation.

The first move is the one most often skipped. Without it, the block
starts with a conclusion before the reader knows what it is about.

These moves determine the length, not a word budget. If there is no
non-obvious decision to explain, keep the context short rather than
inventing one.

A fact about the implementation history earns its place when it explains
something the reader should understand or expect:

- Incomplete: “The collections were public until late in this change.”
- Useful: “The reconciler used to modify the collections directly, which
  is why changes now go through methods that preserve the map's rules.”

## Put the explanation where it belongs

Three levels: block, file, line. Use the highest level that serves.

- Block context explains a decision that shapes the group of changes.
- A file note explains why that file participates in the change or which
  session decision shaped its implementation.
- A line note explains a decision whose consequences are local to a
  specific passage.

Do not repeat the same explanation at every level. Give each note the
context needed to stand on its own, but let it explain the decision
specific to the code beside it.

## Write file notes

A file note connects the file to the change through context the code
does not provide.

Explain why the file exists or changed, which responsibility it received,
or which constraint shaped its implementation. Include the reason
established in the session rather than merely describing the diff.

Use one sentence when it carries the explanation. Add another when the
reader needs it to understand the reason or consequence.

## Write line notes

A line note is often encountered by someone who has not read the block.
Name its subject and provide enough context to understand it directly.

Two things belong together:

1. The implementation choice and the session evidence explaining it.
2. The consequence or boundary that choice creates.

Do not stop at “this was intentional”. Explain what was decided and why.
Do not conclude that the choice is correct, preferable, or sufficient.

Attach a line note to the passage that embodies the decision. Use a
range rather than a single line when the explanation concerns several
lines, and avoid extending the range into unrelated code.

A few explanations grounded in the session are worth more than many
descriptions of the code.

If a note only restates the implementation, remove it. If its reasoning
applies to the whole block, move that explanation to the block context.

Do not invent a decision to fill an otherwise unexplained file.

## Skim marks files the reviewer can skip

A skim marker means the reviewer can skip reading this file without
losing the understanding needed to review the implementation.

The map should concentrate the reviewer's attention on the files that
carry the change: its behavior, decisions, and reasons. Files that only
follow those decisions mechanically can be marked as skim.

Ask of every file:

If the reviewer skips this file entirely, will they miss anything
needed to understand the implementation or a decision behind it?

If the answer is no, the file is a candidate for skim.

For example, a field rename may require changing twenty consumers.
The decision and its implications belong in the main reading path.
Consumers that only adopt the new name can be skipped once that
decision is understood.

Skim is not merely “read this faster”. It means the map does not
require the reviewer to open this file.

Inspect the file before making that recommendation. If it contains
a separate behavior change or a decision the reviewer needs to
understand, keep it in the main reading path.

The skim reason states why the file can be skipped:
“Only updates consumers to use the renamed response field.”

This is a recommendation about what the reviewer needs to read,
not a claim that the file is correct or free of defects.

## Keep skim attached to its context

Attach a skimmed file to the block whose change caused it, using
`--block`, unless it genuinely belongs to no block.

The reviewer should encounter the decision and then its propagation.
Unattached skim entries lose that relationship.

Do not create a block whose only explanation is that its files are
mechanical. Associate those files with the decision they follow.

A file that does not fit an existing block is not automatically skim.
Give a separate change its own context when necessary.

## Write a concrete skim reason

The reason must let the reviewer understand the recommendation and
decide whether to follow it.

- Vague: “Mechanical change.”
- Concrete: “Replaces the renamed field in the request payload.”
- Vague: “Safe to skip.”
- Concrete: “Regenerated lockfile after adding the HTTP client.”

Name the change directly. Include enough context for the reason to
make sense without access to the implementation session.

A file you have not inspected must not be marked as skim. If it
contains a substantive change alongside mechanical edits, keep that
change in the main reading path and explain it at the appropriate level.

## Write for someone arriving without context

Explain the change as you would to someone sitting beside you who
knows the repository but did not participate in the implementation.

Start with the fact the reader needs to understand. Introduce the
reasoning after its subject is clear.

Do not open with an aphorism or a conclusion whose premise is missing.

- Unclear: “Published does not mean finished.”
- Concrete: “Published comments remain in the local review until the
  reviewer closes them, so sending a comment does not remove the
  question from the author's working list.”

Use complete sentences with explicit subjects and verbs. A stack of
labels such as “separate responsibilities, injected dependencies,
thin transport” does not explain what happened or why.

## Introduce the vocabulary

Name a concept before relying on it. If a term could mean two things,
make the intended meaning explicit.

A note must not depend on private vocabulary from the session.
Explain the relevant constraint or decision instead of referring to
an internal nickname, an earlier discussion, or “the approach we chose”.

Write directly in the intended output language, using natural phrasing.
Avoid expressions that only make sense as literal translations.

## Name blocks concretely

A block title is a short phrase naming what the reader will understand
there. It should work in the sidebar without needing the block context
to explain it.

Prefer “How notes follow changing code” to an abstract slogan.

The title names the subject. The context explains its place in the
change, the implementation choice, and the reason behind it.

## Let the explanation determine its length

Use enough detail to connect the implementation to its reason and
consequence. Do not remove necessary context to satisfy a word budget.

When there is no additional decision to explain, keep the text short.
Do not add filler to make a block look more substantial.

Read the explanation back as a reviewer:

- Is its subject clear?
- Does it make sense without the implementation session?
- Does it explain what was done and why, without judging the choice?
- Does each sentence contribute information needed to understand it?

## Verify the map before handing it over

Run:

    farol map check
    farol map show

`map check` must pass. Every file in the review scope must belong to
a block or be marked as skim, and pending note decisions must be resolved.

Coverage alone does not make the map useful. Read the complete output
of `map show` as someone who did not participate in the implementation.

Check that:

- The first block explains what the change is for.
- Each block gives enough context to be understood independently.
- The reading order follows dependencies of understanding.
- Tests appear immediately after the implementation they cover.
- Explanations connect implementation choices to the available evidence.
- Decisions recovered from the session include the context the reviewer
  cannot access.
- Inferred reasons are identified as inference.
- Notes explain choices and their consequences without endorsing them
  or turning into review findings.
- Skimmed files can be skipped without losing necessary understanding,
  and their reasons explain why.
- Mechanical changes are not disguised as blocks with nothing to explain.
- No file was marked as skim without being inspected.
- Titles, terminology, and sentences make sense without private
  vocabulary from the session.

Correct the map wherever these checks expose missing context,
misleading grouping, repetition, or unsupported explanations.

Do not add explanations merely to fill space. Do not remove context
the reviewer needs merely to make the map shorter.

## Respect running reviews

Before starting, reconfiguring, or stopping a review, inspect:

    farol servers

The list includes reviews from other repositories and worktrees.
Identify the intended review by its worktree, branch, and comparison.
Do not assume that a running server belongs to the task you are handling.

Run `farol serve` from the worktree being reviewed, with the intended
comparison options.

If that worktree already has a server, `serve` reuses it and updates
its configuration. Repeated invocation is therefore not always a
no-op: changing the branch or comparison changes what its reader sees.

Confirm that the existing instance is the review you intend to update.
Leave unrelated reviews alone.

Let Farol choose an available port for a new instance. Do not stop
another review to free a preferred port.

Use the URL printed by the command. macOS opens the browser unless
`--no-open` is supplied; Linux prints the URL without opening it.

## Stop only the intended instance

Use the Farol CLI and the port identified in the server list:

    farol servers stop <port>

Do not use `kill`, `pkill`, `killall`, or process-name matching to stop
Farol servers. Those commands can affect other reviews and bypass
Farol's registry management.

Do not use `farol servers stop --all` unless the user explicitly
requested stopping all running reviews.

When restarting an instance, stop only that review and start it again
from the same worktree with the intended comparison.
