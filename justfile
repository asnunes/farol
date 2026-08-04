# The frontend has to be built before cargo, or rust-embed bakes a stale copy
# into the binary. Nothing enforces that ordering except this file.

default: check

build:
    cd web && npm ci --silent || npm install --silent
    cd web && npm run build
    cargo build --release

dev:
    @echo "run 'cd web && npm run dev' in another terminal, then:"
    cargo run -- serve

test:
    cargo test
    cd web && npm test

# The one architectural rule worth enforcing mechanically: HTTP lives in
# server/, and nothing else knows it exists.
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

check: layers test
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check
