# Working on farol

`README.md` says what farol is for. This file says how the code is built and
what to keep doing.

**Everything is in English** — code, comments, CLI output, error messages, UI
copy, commit messages. Conversation about the code may be in any language; the
artefact is not.

## Commands

```bash
just build     # vite build, then cargo build --release
just test      # cargo test + vitest
just layers    # the one architectural rule, enforced
just check     # layers, tests, clippy -D warnings, fmt --check
```

`just check` is the gate. Do not leave work with it failing.

## Shape

Three parts, each split into four layers, plus two general layers on top.

```
src/
├── cmd/         CLI entry points and the composition root
├── server/      HTTP, SSE, embedded assets
├── shared/      errors, workspace and store paths
├── diff/        derived from git, never persisted
├── map/         blocks, order, notes — written by the skill
└── progress/    what has been read — written by the server
```

The parts were chosen by **who writes them**: the skill writes `map`, the server
writes `progress`, nobody writes `diff`. That is the same line that separates the
files on disk, which is why two processes never contend for one of them.

Inside each part:

| layer | holds | may depend on |
|---|---|---|
| `domain` | entities, value objects, invariants, **port traits** | nothing but `shared` |
| `application` | use cases, orchestration | its own `domain` |
| `infra` | implementations of the ports | its own `domain` |
| `presentation` | rendering for humans, models for the wire | its own `domain` |

**Ports live in `domain`, not `application`.** Then the dependency points inward
in every case and there is nothing to decide twice.

**The one rule machine-checked:** nothing outside `src/server/` may reference
`axum`. `just layers` enforces it. If a second rule ever earns that treatment,
add it there rather than to a document nobody re-reads.

## Patterns to follow

### Behaviour hangs off the type that owns it

Prefer a method to a standalone function. If a function's first parameter is the
thing it is really about, it wants to be a method on that thing.

```rust
// yes
range.shift(&hunks)
session.derive()?
MapReport { map, behind }.to_string()

// no
shift_range(from, to, &hunks)
derive(&source, &repo)?
render_map(&map, behind)
```

Free functions are for genuinely typeless helpers, and they should be rare
enough to notice. `position_from(before, after)` is one: it turns two CLI flags
into a `Position` and belongs to neither.

### Parse at the edge; do not validate in the middle

A concept with invariants gets a type, and that type enforces them at
construction. Raw strings are turned into proven values **once, in `cmd`**,
where raw input arrives. Everything below takes the proven type.

| type | proves | built by |
|---|---|---|
| `Slug` | a block name that is kebab-case and non-empty | `Slug::parse` |
| `ReviewPath` | a path under review, carrying the file's length | `DiffSource::review_path` — the only constructor |
| `LineRange` | a span that is non-empty and starts at 1 or later | `LineRange::parse` / `new` |

This is stronger than validating inside the use case. A check inside can be
skipped by a caller that forgets; **a type cannot be skipped, because the wrong
call does not compile.** `session.add_file(&Slug, &ReviewPath, ..)` cannot be
handed a hallucinated path, and its two arguments cannot be transposed the way
two `&str` invite.

`ReviewPath` carries the line count because it is known at the same moment, so
`range.require_within(&path)` needs no access to the diff source.

### Use cases live in `application`, not in the entry point

An entry point parses arguments and prints. It does not decide what is allowed,
and it does not compose domain operations.

```rust
// yes — cmd/block.rs
ctx.report(
    format!("Added block '{slug}'."),
    ctx.map().add_block(&slug, &title, &context, position, &paths),
)

// no — validation and composition in the entry point
for path in &paths { ctx.map().require_in_scope(path)?; }
ctx.edit(|map| { map.add_block(..)?; for p in &paths { map.add_file(..)?; } Ok(()) })
```

The reason is not tidiness. **Validation that lives in the caller is validation
every future caller can skip** — an HTTP route added later would have to
remember `require_in_scope` on its own. Inside the use case it cannot be
forgotten.

### Dependencies are injected, always

Nothing constructs what it uses. Collaborators arrive through the constructor,
as trait objects, so any of them can be replaced by a fake.

```rust
// yes — receives its collaborators
impl<'a> MapSession<'a> {
    pub fn new(source: &'a dyn DiffSource, repo: &'a dyn MapRepository) -> Self
}

// no — reaches out and builds one
impl MapSession {
    pub fn new() -> Self { Self { repo: JsonMapRepository::new(...) } }
}
```

**`src/cmd/wiring.rs` is the composition root and the only place a concrete
implementation is named.** `Ctx::from_workspace` chooses `GixSource`,
`JsonMapRepository` and `JsonProgressRepository`; `Ctx::new` takes them. If you
find yourself writing `SomeConcrete::new()` anywhere else, the dependency wants
to be a parameter instead.

Borrow (`&dyn`) when the collaborator outlives the call; share (`Arc<dyn>`) when
it crosses into the server's state.

### Ports stay synchronous

For local file IO, async in Rust is mostly theatre: `tokio::fs` is a threadpool
wrapper, so you would pay the syntax and get no concurrency. Meanwhile every
domain and application test would need a runtime.

The known cost is real: a synchronous port blocks a tokio worker, and
`DiffSource` is genuinely expensive — it reads both trees and diffs in process.
If that ever hurts, the fix is `tokio::task::spawn_blocking` **at the call
site**, which is local and does not touch the trait. That is what keeps this
decision reversible.

The one thing that would overturn it is a port that is genuinely remote — a map
stored on a server rather than on disk. That is the deferred "the map travels"
idea, and it would likely arrive as a new port anyway.

### Reports implement `Display`

Anything printed borrows what it prints and implements `Display`, so callers
hand it to `print!` and nothing builds a `String` it does not need:
`ScopeReport`, `MapReport`, `OrphanReport`, `CheckSummary`.

The alternative considered was a component or macro layer in the spirit of ink,
which would make layout declarative instead of hand-rolled `writeln!`. It lost
on a fact specific to farol: **`map show` and `scope` exist for the skill to
read.** Their consumer is a model, which does not care about alignment, so a
layout engine would be investment in the wrong place. If human-facing output
ever appears — a map worth pasting into a pull request — that calculation
changes.

If the `write!` noise becomes the problem, the cheap middle is two or three
writer primitives, not a component layer.

### The view model is assembled on the server

`ReviewView::build` applies the reading rules — a file is shown once under the
earliest block that holds it, carrying the notes and tags from every block it
belongs to. The frontend renders; it does not decide.

## Tests

Three layers, and each covers something the others cannot.

**Unit tests live beside the code**, in a `#[cfg(test)] mod tests` at the bottom
of the file. Ordering, re-anchoring, view assembly and progress invalidation are
faster and clearer over fakes than over commits.

**Route tests live in `src/server/mod.rs`** and drive the real `Router` through
`tower::ServiceExt::oneshot`, over in-memory repositories. Nothing listens on a
port. They cover what a unit test cannot: that the wiring, the status codes and
the JSON shape the browser depends on are what they claim.

**Integration tests live in `tests/cli.rs`** and drive the real binary against
real repositories. They cover what only git can prove: how git dirs resolve
inside a worktree, what a merge base returns once the base branch has moved,
whether a command actually exits non-zero.

A fake must honour its port's contract. `FakeDiffSource` refuses a path outside
the review because `GixSource` does; a fake that were more permissive would let
tests pass over behaviour that does not exist. That exact bug was caught by the
route tests on their first run.

**Fakes live in `src/testing.rs`**, one place that knows how to stand in for git
and for storage. Extend `FakeDiffSource` rather than writing a new stand-in.

```rust
let source = FakeDiffSource::with_paths(&["a.rs"])
    .on_commit("new")
    .with_ancestors(&["old"])
    .changed_between("old", "new", "a.rs", vec![hunk(1, 0, 5)]);
```

**Test names are sentences about behaviour**, not about method names:

```rust
fn a_note_far_from_the_change_slides_instead_of_being_deactivated()
fn serve_refuses_to_start_without_a_map()
fn a_file_in_two_blocks_is_rendered_once_in_the_earlier_one()
```

Reading the test list should describe the contract. `fn test_shift_range_2()`
describes nothing.

**Assert on the reason, not only the value.** When an assertion encodes a
decision, put the decision in the failure message:

```rust
assert_eq!(
    map.orphans[0].snapshot, "const timeout = 180;",
    "the snapshot is what identifies the note afterwards, not the old range"
);
```

## Comments

Write why, not what. The code already says what.

Good: *"Hashing the diff would reopen the whole branch on a rebase that only
shifted context lines."*
Useless: *"Hash the file contents."*

Doc comments on a type explain the decision it embodies — why it exists at all,
what alternative was rejected. Inline comments mark the places where a reader
would otherwise ask "why like this?".

If a comment restates the line under it, delete it.

## Errors

One `Error` enum in `shared/error.rs`, with a variant per situation and the
message written for whoever hits it. Messages say what went wrong **and what to
do**:

```
no map for branch fix/signing-handoff
Run the review-map skill in the session that implemented this change.
```

Rejections aimed at the skill carry the way out — near-miss paths for a
hallucinated path, the existing slugs for an unknown one. That is what turns a
rejection into a self-correction instead of a dead end.

## Things deliberately not done

Do not add these back without a reason that has changed:

- **Review comments.** v0 is for understanding, not commenting. When they come,
  they must be a separate collection — the skill regenerates notes wholesale and
  must never touch hand-written text.
- **Automatic re-anchoring by text matching.** Deactivating and re-offering hands
  the judgement to whoever has the context. Guessing puts a confident note on
  unrelated code.
- **Migrations for stored state.** It dies with the worktree. Version mismatch
  discards and says so.
- **A config file, cross-compilation, big-PR strategies.** Later, if the need is
  real.
