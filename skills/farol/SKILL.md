---
name: farol
description: Apply Farol's principles for organizing review maps, writing implementation context, choosing reading order, and identifying files the reviewer can skip.
---

# Review maps with Farol

Farol shows a change in the order that makes its implementation understandable,
carrying the context the diff does not.

A map contains blocks, explanations, file and line notes, and the files
in focus.
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
across folders.

A file belongs to one block. When it changed for two stories, put it in the
block the reader needs it for first, and leave a line note on the lines that
belong to the other story, naming it. Farol refuses a file already in another
block and says which one.

Do not group by folder or by the file's role — controller, service,
repository. That grouping does not depend on the change: two completely
different changes would produce the same split.

Every file goes into a block. A mechanical change belongs to the block
whose decision caused it, where the reader meets the decision first and
its propagation after. A file with no story at all goes in `outros`.

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
or conversation workflow appropriate to the task, or in the lens that
asks for it (see Write the lenses).

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

## Put in focus what is worth opening

On a review with lenses, every file starts folded: the reader learns
the change from the lenses and opens the code when they want to check
it. Put in focus the files that deserve to be opened anyway, and say
why:

    farol file focus <slug> <path> --reason '…'
    farol file unfocus <slug> <path>

A file earns focus when reading its code pays: it holds the rule the
change is about, it is dense enough that a summary loses what matters,
or something in it is wrong in a way the reader should see for
themselves. The reason names which, concretely, so the reader can
decide whether to follow it:

- Vague: "Important file."
- Concrete: "Holds the whole retry rule; the rest only calls it."
- Vague: "Needs attention."
- Concrete: "A retry here can resend a charge the provider accepted."

Focus few. Focus on every file is no focus at all, and the point of the
lenses is that reading them costs less than reading the code. Inspect a
file before putting it in focus, and never do it to make up for a lens
that explains it badly: fix the lens.

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
a block, and pending note decisions must be resolved.

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
- The files in focus are few, and each reason says
  concretely why opening it pays.
- Mechanical changes sit in the block of the decision they follow, not
  in a block of their own with nothing to explain.
- Titles, terminology, and sentences make sense without private
  vocabulary from the session.

Correct the map wherever these checks expose missing context,
misleading grouping, repetition, or unsupported explanations.

Do not add explanations merely to fill space. Do not remove context
the reviewer needs merely to make the map shorter.

## Write the lenses

Lenses let the reviewer understand the change before reading the code:
what the pull request does, where its risk is, what was checked, which
tests exist. Each lens is a title and a prompt in a group: the pull
request, each block, or each file the prompt applies to. The reviewer
chooses which lenses exist and what each asks, so ask farol instead of
assuming the list:

    farol lens pending

It lists every lens that is missing or outdated, with its prompt, the
command for each target, and the rules every answer is held to. Every
time you update the map, review the outdated ones: if what a lens says
still holds for the code as it is now, keep it with
`farol lens keep <id> [--block <slug> | --file <path>]`; if not, rewrite
it. Write the missing ones, following their prompt:

    farol lens write <id> --text '…'                  # the pull request
    farol lens write <id> --block <slug> --text '…'   # a block
    farol lens write <id> --file <path> --text '…'    # a file

Keep each answer to a few lines: the point of a lens is that reading it
takes less time than reading the code. Write them from what the session
knows, in the map's language. Cite code in backticks as `path:from-to`,
adding `(old)` for a line the change removed; a cited line that does not
exist is refused. A file lens is offered for every file in the review:
write an empty text for a file it does not apply to. When a lens asks for
files a person should read themselves, pass each with `--read <path>`.

Lenses are where judgment goes. Unlike the map, a lens such as Risk or
Evidence may assess the change, because the reviewer asked for it there.
Writing a lens never changes, commits or publishes code.

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

### Respect the user's browser preference

Use `farol serve` with the intended comparison options and no browser flags
by default. Return the URL printed by the command. Browser opening follows
the user's saved preference, configurable through `farol config set
open-browser true|false` or **Open browser automatically** in the browser's
**Settings**.

The flags override that saved preference for the current invocation:

- `--open` requests opening the browser even when the saved preference disables it.
- `--no-open` prevents opening the browser even when the saved preference enables it.

Use either flag only when the user explicitly requests the corresponding
browser behavior, such as "start Farol and open the browser" or "start Farol
without opening the browser". Never infer an override from "start Farol",
"open Farol", "show the review", or other wording that does not explicitly
mention opening or not opening the browser.

Do not ask about browser opening when it was not mentioned. Do not change
the saved preference or open the browser separately unless explicitly
requested. Apply the same rule when reusing or restarting a review.

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

## Answer threads sent from Farol

A reviewer can send a thread from the review page straight into your
session. It arrives as a message that names Farol and carries the
thread's id, its location and text, and any conversation so far.

Acknowledge it before you start, so the page shows you are on it:

    farol thread ack <id>

Answer through Farol, not only in your own reply, because the reviewer
reads the answer on the page:

    farol thread reply <id> --text '…'

Use single quotes: inside double quotes a backtick runs a command. Say
what you changed when you changed code. Do not close the thread unless
the reviewer asks; the conversation stays open until they close it.

## Write a pending comment when asked

In a peer review the reviewer can ask for a thread's comment for the pull
request. The message says so and names the command. Acknowledge it the
same way, then write the comment:

    farol pending write <id> --text '…'

It goes into the reviewer's pending review on GitHub, which only they see
until they submit the review, and replaces the thread's previous one. Write
it for the author of the pull request, following the instruction in the
message. Never submit the review, approve, or post anything else on the
pull request yourself.

## Follow the mode rule

Every message ends with the review mode's rule, and it outranks any other
instruction in the message. In a self review you may change the code. In a
peer review do not change, commit, or push code in that repository.
