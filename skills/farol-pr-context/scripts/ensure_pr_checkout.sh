#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: ensure_pr_checkout.sh <pr-number-or-url>" >&2
  exit 64
fi

pr_selector=$1

if ! command -v git >/dev/null 2>&1; then
  echo "error: git is required" >&2
  exit 69
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "error: GitHub CLI (gh) is required" >&2
  exit 69
fi

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
  echo "error: the current directory is not inside a Git repository" >&2
  exit 69
}

cd "$repo_root"

read -r head_ref head_oid < <(
  gh pr view "$pr_selector" --json headRefName,headRefOid --jq '[.headRefName, .headRefOid] | @tsv'
)
current_ref=$(git symbolic-ref --quiet --short HEAD || true)
current_oid=$(git rev-parse HEAD)

if [[ "$current_ref" == "$head_ref" && "$current_oid" == "$head_oid" ]]; then
  if [[ -n "$(git status --porcelain=v1)" ]]; then
    echo "status=aligned"
    echo "worktree_dirty=true"
  else
    echo "status=aligned"
    echo "worktree_dirty=false"
  fi
  echo "head_ref=$head_ref"
  echo "head_oid=$head_oid"
  exit 0
fi

if [[ -n "$(git status --porcelain=v1)" ]]; then
  echo "error: checkout is required, but the worktree has local changes" >&2
  git status --short >&2
  exit 65
fi

gh pr checkout "$pr_selector"

read -r refreshed_head_ref refreshed_head_oid < <(
  gh pr view "$pr_selector" --json headRefName,headRefOid --jq '[.headRefName, .headRefOid] | @tsv'
)
checked_out_ref=$(git symbolic-ref --quiet --short HEAD || true)
checked_out_oid=$(git rev-parse HEAD)

if [[ "$checked_out_ref" != "$refreshed_head_ref" ]]; then
  echo "error: expected branch '$refreshed_head_ref', found '$checked_out_ref'" >&2
  echo "The PR branch may already be checked out in another worktree." >&2
  exit 70
fi

if [[ "$checked_out_oid" != "$refreshed_head_oid" ]]; then
  echo "error: branch '$checked_out_ref' is not aligned with PR head '$refreshed_head_oid'" >&2
  echo "Refusing to force-reset or discard local commits." >&2
  exit 70
fi

echo "status=checked_out"
echo "worktree_dirty=false"
echo "head_ref=$checked_out_ref"
echo "head_oid=$checked_out_oid"
