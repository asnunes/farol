#!/usr/bin/env bash
set -euo pipefail

# Packaging runs natively so the version check also executes the built binary.
target=${1:?usage: package-release.sh <target>}
binary="$(pwd)/target/$target/release/farol"
version=$("$binary" --version)
version=${version#farol }
if [[ ${GITHUB_REF_TYPE:-} == tag && ${GITHUB_REF_NAME:-} != "v$version" ]]; then
    echo "release tag must match Cargo.toml: expected v$version" >&2
    exit 1
fi

archive="farol-v$version-$target.tar.gz"
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
mkdir -p dist
cp "$binary" "$staging/farol"
cp README.md "$staging/README.md"
tar -czf "dist/$archive" -C "$staging" farol README.md
(
    cd dist
    shasum -a 256 "$archive" > "$archive.sha256"
)
echo "dist/$archive"
