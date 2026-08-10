# The frontend has to be built before cargo, or rust-embed bakes a stale copy
# into the binary. Nothing enforces that ordering except this file.

default: check

# Build the frontend, then the binary that embeds it.
build:
    cd web && npm ci --silent || npm install --silent
    cd web && npm run build
    cargo build --release

# Put `farol` on your PATH, pointing at this checkout.
#
# A symlink rather than a copy: `just build` then updates the installed binary
# too, which is what you want while the tool is still being written. `cargo
# install` would copy — and worse, it would skip the frontend build and embed
# whatever happens to be in web/dist.
install: build
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p ~/.local/bin
    ln -sf "$(pwd)/target/release/farol" ~/.local/bin/farol
    echo "farol -> $(readlink ~/.local/bin/farol)"

# Serve with the frontend proxied from Vite instead of embedded.
dev:
    @echo "run 'cd web && npm run dev' in another terminal, then:"
    cargo run -- serve

# Run every suite: Rust unit, Rust integration, and web.
test:
    cargo test
    cd web && npm test

# `testing.rs` is excluded because measuring the fakes tells you nothing about
# the code they stand in for.
#
# Show where the tests are not looking.
coverage:
    cargo llvm-cov --summary-only --ignore-filename-regex 'testing\.rs'
    cd web && npx vitest run --coverage --coverage.reporter=text

# Real references only — a comment explaining why something has to be Sync for
# axum's sake is not a dependency on axum.
#
# Check that HTTP stays in server/ and nothing else imports axum.
layers:
    #!/usr/bin/env bash
    set -euo pipefail
    # Real references only — a comment explaining why something has to be Sync
    # for axum's sake is not a dependency on axum.
    hits=$(grep -rn --include='*.rs' -E '(^|[^/*[:alnum:]_])axum::|^[[:space:]]*use axum' src --exclude-dir=server \
           | grep -vE ':[[:space:]]*(//|\*)' || true)
    if [ -n "$hits" ]; then
      echo "$hits"
      echo "axum leaked outside src/server/" >&2
      exit 1
    fi
    echo "layers ok"

# What has to pass before a commit.
#
# `tsc` is here and not in `npm test` because vitest does not typecheck: a test
# can pass while naming a field that does not exist.
check: layers test
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check
    cd web && npx tsc -b
