# The frontend has to be built before cargo, or rust-embed bakes a stale copy
# into the binary. Nothing enforces that ordering except this file.

default: check

# Build the frontend, then the binary that embeds it.
build: web-dist
    cargo build --locked --release

# Native release packages are built and exercised on their matching runners.
package target: web-dist
    cargo build --locked --release --target {{target}}
    bash scripts/package-release.sh {{target}}

smoke-package archive:
    bash scripts/smoke-release.sh "{{archive}}"

# A symlink rather than a copy: `just build` then updates the installed binary
# too, which is what you want while the tool is still being written. `cargo
# install` would copy — and worse, it would skip the frontend build and embed
# whatever happens to be in web/dist.
#
# Put `farol` on your PATH, pointing at this checkout.
install: build
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p ~/.local/bin
    ln -sf "$(pwd)/target/release/farol" ~/.local/bin/farol
    echo "farol -> $(readlink ~/.local/bin/farol)"

# Only removes a link pointing at this checkout: a `farol` installed from
# somewhere else is somebody else's, and silently deleting it would be a
# surprise.
#
# Take `farol` back off your PATH.
uninstall:
    #!/usr/bin/env bash
    set -euo pipefail
    link=~/.local/bin/farol
    if [ ! -e "$link" ] && [ ! -L "$link" ]; then
      echo "nothing installed at $link"
      exit 0
    fi
    target=$(readlink "$link" || true)
    if [ "$target" != "$(pwd)/target/release/farol" ]; then
      echo "$link does not point at this checkout (points at: ${target:-a real file})" >&2
      echo "leaving it alone — remove it by hand if you meant to" >&2
      exit 1
    fi
    rm "$link"
    echo "removed $link"

# Serve with the frontend proxied from Vite instead of embedded.
dev:
    @echo "run 'cd web && npm run dev' in another terminal, then:"
    cargo run -- serve

# Run every suite: Rust unit, Rust integration, and web.
test:
    cargo test --locked
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

# Split in two because the two halves have nothing to say to each other, and CI
# runs them side by side. Locally you want both, which is what `check` is.
#
# What has to pass before a commit.
check: check-rust check-web

# Everything the Rust side has to satisfy.
check-rust: layers
    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
    cargo fmt --check

# `tsc` is here and not in `npm test` because vitest does not typecheck: a test
# can pass while naming a field that does not exist.
#
# Everything the web side has to satisfy.
check-web:
    cd web && npm test
    cd web && npx tsc -b

# Without it `tests/server.rs` still passes, by taking the branch that asserts
# farol says what to build. So a CI that skips this leaves the page itself
# covered by nothing.
#
# The frontend the binary embeds, which the integration tests serve.
web-dist:
    cd web && npm ci
    cd web && npm run build
