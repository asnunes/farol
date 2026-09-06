#!/usr/bin/env bash
set -euo pipefail

archive=${1:?usage: smoke-release.sh <archive.tar.gz>}
archive_dir=$(cd "$(dirname "$archive")" && pwd)
archive_name=$(basename "$archive")
(
    cd "$archive_dir"
    shasum -a 256 -c "$archive_name.sha256"
)

work=$(mktemp -d)
port=""
binary="$work/package/farol"
# This registry and configuration cannot reach the maintainer's running reviews
# or credentials. Cleanup uses the same public command a user would run.
export FAROL_STATE_DIR="$work/state"
export XDG_CONFIG_HOME="$work/config"
cleanup() {
    if [[ -n "$port" ]]; then
        "$binary" servers stop "$port"
    fi
    rm -rf "$work"
}
trap cleanup EXIT
mkdir -p "$work/package"
tar -xzf "$archive_dir/$archive_name" -C "$work/package"
"$binary" --version

# No checkout, web/dist, Node, or build tree supplies the extracted server's UI.
git init -q --initial-branch=main "$work/repo"
cd "$work/repo"
git config user.name "Farol package test"
git config user.email "package-test@example.invalid"
git config commit.gpgsign false
printf 'before\n' > example.txt
git add example.txt
git commit -qm initial
git checkout -qb package-review
printf 'release smoke change\n' > example.txt
git add example.txt
git commit -qm change
"$binary" scope
"$binary" map derive
"$binary" block add example --title "Packaged review" --context "The release binary serves its own assets." example.txt
"$binary" map check

if [[ $(uname -s) == Linux ]]; then
    started=$("$binary" serve --port 0)
else
    started=$("$binary" serve --port 0 --no-open)
fi
printf '%s\n' "$started"
port=$(printf '%s\n' "$started" | sed -n 's/.*http:\/\/127\.0\.0\.1:\([0-9][0-9]*\).*/\1/p')
[[ -n "$port" ]]
url="http://127.0.0.1:$port"
curl -fsS --max-time 10 "$url/api/review" > "$work/review.json"
jq -e '.branch == "package-review" and .totalFiles == 1 and .blocks[0].title == "Packaged review"' "$work/review.json"
curl -fsS --max-time 10 "$url/api/file?path=example.txt" > "$work/diff.json"
jq -e '[.hunks[].lines[].content] | index("release smoke change") != null' "$work/diff.json"
curl -fsS --max-time 10 "$url/" > "$work/index.html"
script=$(sed -n 's/.*src="\([^"]*\.js\)".*/\1/p' "$work/index.html")
style=$(sed -n 's/.*href="\([^"]*\.css\)".*/\1/p' "$work/index.html")
[[ -n "$script" && -n "$style" ]]
curl -fsS --max-time 10 "$url$script" > "$work/app.js"
curl -fsS --max-time 10 "$url$style" > "$work/app.css"
[[ -s "$work/app.js" && -s "$work/app.css" ]]
! cmp -s "$work/index.html" "$work/app.js"
! cmp -s "$work/index.html" "$work/app.css"
if [[ $(uname -s) == Linux ]]; then
    reused=$("$binary" serve)
else
    reused=$("$binary" serve --no-open)
fi
[[ "$reused" == *"updated the review at $url"* ]]
"$binary" servers stop "$port"
port=""
echo "Packaged CLI, embedded assets, API, server reuse, and shutdown passed."
