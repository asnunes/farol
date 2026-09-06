# Developing Farol

## Requirements

- Git and a C/C++ build toolchain: Xcode Command Line Tools on macOS, or
  `build-essential` and `pkg-config` on Ubuntu.
- [rustup](https://rustup.rs/). `rust-toolchain.toml` selects Rust 1.97.1,
  Clippy, and rustfmt.
- Node 22.23.2 with npm, also recorded in `.node-version`.
- Just 1.57.0.

## Build and install

```bash
git clone https://github.com/asnunes/farol.git
cd farol
rustup show
just build
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/farol "$HOME/.local/bin/farol"
export PATH="$HOME/.local/bin:$PATH"
farol --version
```

`just build` runs `npm ci`, builds the frontend, then builds Rust with
`--locked`. The ordering matters because the executable embeds the frontend.

For an actively developed checkout, `just install` creates a symlink to
`target/release/farol` instead of copying it. Subsequent builds update that
installation. `just uninstall` removes the link if it still points here.

## Run with frontend changes

In one terminal:

```bash
cd web
npm run dev
```

In another, from the repository root:

```bash
cargo run -- serve
```

The repository needs a review map; see [writing a map](usage.md#write-a-map).
On Linux, open the printed URL manually.

## Check a change

```bash
just check
```

This runs the Rust and web tests, Clippy, rustfmt, TypeScript, and the rule that
HTTP dependencies stay inside `src/server/`. CI runs the Rust and web checks in
parallel. For a fresh checkout, run `just build` first so server tests have the
frontend to serve.

Rust unit tests live beside the implementation. CLI and HTTP integration tests
are in `tests/`; frontend tests are beside the components and hooks.

See the [release guide](releases.md) for native packaging and smoke checks.
