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
or mark mechanical changes with `farol skim add`. `map check` fails until all
changed files are assigned and pending note decisions are resolved.

## Read and comment

| Key | Action |
|---|---|
| `j` / `k` or arrow keys | Move between files |
| `n` | Next unread file |
| `;` or Space | Mark the current file read |
| `[` / `]` | Move between blocks |
| `?` | Show shortcuts |

Drag down line numbers or click `+` to comment. Expanded context between hunks
is readable but does not accept comments: GitHub comments must land inside the
diff.

Both sides of the diff take comments. The numbers on the left are the code the
change removed, which is the only side a file deleted whole still has; the
numbers on the right are the file as it now reads. In split view the column you
drag decides the side; in unified view a removed line is counted on the old
numbering and everything else on the new one. A span stays on one side — a drag
that reaches the other column stops where its own side stops. The side travels
with the comment and is published as `LEFT` or `RIGHT`.

Comments can also be managed through the CLI:

```bash
farol comment list
farol comment add src/retry.rs 82-116 --text "Why is this ordering deliberate?"
farol comment add src/gone.rs 2-4 --side old --text "Where did this go?"
```

`--side` is `new` by default, which is the file as it now reads. Ranges on the
old side are the line numbers of the file before the change.

`farol comment close ID` removes a comment once it is answered. In the browser,
closing asks for confirmation because local comments are not recoverable from Git.

## Send to GitHub

Use **Send review** in the browser. Farol explains any missing token, push, or
pull request. From the CLI:

```bash
farol github status
farol github review --comment
farol github review --request-changes --summary "The retry window needs a bound."
```

`status` exits nonzero while publishing is unavailable. Configure the token in
the browser, where the destination host is shown. Authorize only a host you trust.

The credential is saved with that host in
`$XDG_CONFIG_HOME/farol/github-credential.json` (default: `~/.config/farol`).
Farol stores one credential at a time, and never uses it for another host,
including during readiness checks. A changed remote requires refreshing the
form before saving. An old `github-token` file has no host authorization and is
not loaded; save the token again through the form.

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
browser unless `--no-open` is passed. Linux only prints the URL.

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
