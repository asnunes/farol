# Using Farol

## Write a map

A map records the order and explanations of a review. Prefer having the coding
session that implemented the change write it while the decisions are still in
context. To write it manually, start on the branch under review:

```bash
farol scope
farol map derive
```

Use the paths reported by `scope`. This example assumes the branch changed
`src/retry.rs`:

```bash
farol block add retry-window \
  --title "Bound the retry window" \
  --context "The first version retried forever, which hid the timeout." \
  src/retry.rs
farol line add retry-window src/retry.rs 82-116 \
  --note "This ordering is deliberate."
farol map check
farol serve
```

Choose line ranges that exist in the file. Add more files with `farol file add`,
or put a file in focus with `farol file focus`. `map check` fails
until all changed files are assigned and pending note decisions are resolved.

## Lenses

Lenses say what the change is before the code. Each is a title and a prompt
in one of three groups: the pull request, each block, or each file it applies
to. Four are on by default, one for each thing that stays with a person when
machines do most of the reading:

- **Overview** (pull request): what it does, why, and the decisions behind it.
- **Risk** (pull request): how risky, the sensitive paths, a pattern others
  will copy, and the files to read yourself.
- **Evidence** (pull request): what ran and what it returned, the problems a
  fresh review finds, and what those checks would miss.
- **Tests** (file): the tests in each file that has them.

Vocabulary, Architecture (per block) and Security come switched off.

The session writes them when it builds or updates the map. On the screen they
are open sections in the reading: the pull request's before the first block,
a block's after its context, a file's under its header. Once the session has
written lenses, every file starts folded and opens when you ask for it; a lens
whose code changed since says Outdated until the map is updated.

Settings lists the lenses by group. Switch one off, edit its title or prompt,
delete it, or add your own. The list is kept in `lenses.json` beside the
settings, for every repository.

```bash
farol lens list                                   # the catalog
farol lens add --group file --title Migrations --prompt '…'
farol lens pending                                # what the session owes
farol lens write tests --file src/retry.rs --text '…'
farol lens keep overview                          # still holds after a commit
```

## Read and comment

| Key | Action |
|---|---|
| `j` / `k` or arrow keys | Move between files |
| `n` | Next unread file |
| `;` or Space | Mark the current file read |
| `[` / `]` | Move between blocks |
| `?` | Show shortcuts |
| `Shift` + `↑` / `↓` on a `+` | Reach for another line to comment on |

Drag down line numbers or click `+` to comment. Without a mouse, tab to the `+`
on a line and hold `Shift` with the up or down arrow: each press reaches one
line further, the rows light up as they do under a drag, and `Enter` opens the
box over everything reached. The reach stays inside the hunk and on the side the
`+` belongs to. Expanded context between hunks is readable but does not accept
comments: GitHub comments must land inside the diff.

Both sides of the diff take comments. The numbers on the left are the code the
change removed, which is the only side a file deleted whole still has; the
numbers on the right are the file as it now reads. In split view the column you
drag decides the side; in unified view a removed line is counted on the old
numbering and everything else on the new one. A span stays on one side — a drag
that reaches the other column stops where its own side stops. The side travels
with the comment and is published as `LEFT` or `RIGHT`.

Each comment starts a **thread**: a conversation about those lines, with the
agent or just to copy. Threads stay local. They can also be managed through the
CLI:

```bash
farol thread list
farol thread add src/retry.rs 82-116 --text "Why is this ordering deliberate?"
farol thread add src/gone.rs 2-4 --side old --text "Where did this go?"
```

`--side` is `new` by default, which is the file as it now reads. Ranges on the
old side are the line numbers of the file before the change.

A thread whose lines the diff no longer shows — the code moved, often because
of the fix it asked for, or the file left the review — is not lost: it moves to
the end of its file's block, or to a block of its own at the end of the page,
and can still be answered and closed. `farol thread list` lists them under
"No longer on the diff". Only a pending comment needs its lines in the diff.

`farol thread close ID` removes a thread once it is answered. In the browser,
closing asks for confirmation because local threads are not recoverable from Git.

### Ask the agent

When the review was opened by `farol serve` from inside a Claude Code or Codex
session, the box a thread is written in gets **Ask {agent}**, which saves it and
sends it in one press, and each thread gets a **Send** button next to **Copy**.
`⌘↵` asks; `⇧⌘↵` only saves. Claude Code receives the message on its session
socket between tool calls, and Codex queues it with `codex queue` until the
current turn ends. Neither interrupts the agent. The tick means the harness took
the message, not that the agent has read it.

The message always says it comes from farol and carries the thread's id,
location, text and the conversation so far, with the two commands the agent
answers through: `farol thread ack ID` when it starts, which marks the thread as
being worked on, and `farol thread reply ID --text '…'`, which puts the answer
under it on the page. That part is fixed. After it comes the instruction set in
`send-instruction`, and last the rule of the review mode, which no instruction
overrides.

**Reply** writes back under a thread, and **Reply & send** hands the whole
exchange to the agent again. The page marks a thread sent to an agent until the
agent answers, and says so when a reply of yours was saved but not sent.

Running `farol serve` again from another session points the open review at that
session. A `farol serve` run from a plain terminal keeps the session already
known. Without a session there is no Send button, and **Copy** works as before.

### Self review and peer review

A **self review** is your own branch: the agent is the one that wrote it and may
change the code. A **peer review** is somebody else's pull request: the agent is
your assistant, every message tells it not to change, commit or push code, and
threads can become pending comments.

Farol tells them apart by the pull request's author, and shows the mode and why
in the top bar, where it can be changed. `farol serve --self-review` or
`--peer-review` sets it from the terminal. Either choice is remembered for the
branch. Without a pull request it is a self review.

### Pending comments

In a peer review a thread can have a **pending comment**: the comment for the
pull request that came out of it. It lives in your pending review on GitHub,
which only you see until you submit the review, so it also shows on the pull
request's page and can be edited there. There are three ways to one:

- **Generate pending comment** asks the agent to write it, following
  `pending-instruction`. The arrow beside it adds an instruction for this one.
- **Add as pending comment** in the box starts the thread with your own words
  as its comment.
- **Use as pending comment** takes the question or a turn of the conversation
  as it is.

The pending comment sits under its thread with **Edit**, **Regenerate** and
**Remove**. A new one is written before the old one is taken back. Writing one
needs the pull request to show the commit you are reading.

```bash
farol pending write ID --text '…'   # what the agent runs
farol pending edit ID --text '…'
farol pending remove ID
```

## Send to GitHub

**Submit review…** in the top bar lists the pending comments that go, says how
many threads stay in farol, and takes the verdict and an optional summary.
Asking for changes needs a summary. On your own pull request only **Comment** is
offered: GitHub does not let an author approve or request changes on their own.

The connection is your GitHub CLI login. Farol asks `gh auth token --hostname
HOST` at each call, so `GH_TOKEN` and `GITHUB_TOKEN` work too, and never stores a
token or accepts one through the page. Without a login the top bar says
**Connect GitHub** with the command to run, and without a pull request it links
to the page that opens one; threads keep working either way. The page looks again
when you come back to its tab, so there is nothing to reload.

The skill `farol-publish-review` does the same from a session. The CLI it uses:

```bash
farol github status
farol github review --comment
farol github review --request-changes --summary "The retry window needs a bound."
farol github ticks
```

`status` lists the pull request, the mode and the pending comments, and exits
nonzero while the pull request cannot be reached. `review` submits the pending
review and ticks the files you read. `ticks` sends only the read marks, and needs
the pull request to show the commit you read.

## Share a map

Export the map after committing the reviewed code:

```bash
farol map export --out review.farol.json
```

The recipient checks out the corresponding code and imports the file:

```bash
farol map import review.farol.json
farol serve
```

Only the map travels. Code comes from Git; comments and reading progress remain
local. Import requires a matching base commit and reports when the recipient's
head differs from the map's head.

## Manage open reviews

```bash
farol servers
farol servers stop 4600
```

Use the port shown for the review you want to stop. `farol serve` runs in the
background and reuses the server for the current worktree. On macOS it opens a
browser by default. Linux only prints the URL.

### Global preferences

```bash
farol config list
farol config show
farol config set open-browser false
farol config set send-instruction "Answer it. Leave it open."
farol config reset send-instruction
farol serve --open
farol serve --no-open
```

`farol config list` shows the settings available on this system, including their
titles, types, optional descriptions, defaults and current values. `farol config
show` shows the values in effect, defaults included. Both the CLI and the review
page use the backend catalog.

`config set` reads the value the way the setting reads it: `true` or `false` for
a switch, the text as written for a text setting, including an empty string.
`config reset` puts a setting back to its default. The file keeps only what you
set, so a default improved in a later release still reaches you.

Preferences live in `$XDG_CONFIG_HOME/farol/config.json`, or
`~/.config/farol/config.json` when that variable is unset. They apply to all
repositories for your user on this machine. No file means automatic browser
opening stays enabled on macOS, and Send uses the default instruction.

The review's **Settings** button reads and writes the same preferences; a text
setting is saved with its **Save** button. The send instruction is read on every
Send, so a change counts on the next press. Changes to opening the browser
apply to future `farol serve` invocations, including when reusing a running
server. `--open` and `--no-open` override the saved value for one invocation;
they cannot be combined. Configuration commands work outside a repository.

Each invocation replaces the previous comparison options. `--port` chooses a
port only for a new server; `--foreground` holds the terminal when starting one.
Separate worktrees have separate servers and stores.

## Comparison and state

- **Base:** `main`, falling back to `master`. The default head is the current
  branch. Detached HEAD is refused because maps belong to branches.
- **Diff:** taken from the merge base. `--direct` compares the two refs directly;
  `--dirty` includes uncommitted work.
- **Refresh:** map and current-branch changes announce an update. The reader
  decides when to load it. `--no-watch` disables these notifications.
- **Progress:** a read file stays read until its contents change. Shifts in diff
  context alone do not reset it.
- **Notes:** changes outside a note shift its range. Changes through a note
  deactivate it for the author to restore or discard during map derivation.
- **Storage:** maps inherit previous versions. Review data lives under the
  worktree's Git directory and is removed with that worktree.
- **Local server:** open the printed URL. Requests must use `127.0.0.1` or
  `localhost` on the listening port; unrelated browser origins are refused.

Use `farol --help` or `farol COMMAND --help` for the full command reference.
