---
name: farol-pr-context
description: Gather factual GitHub PR context for Farol map creation or import. Use within a Farol workflow or when explicitly requested for Farol.
---

# PR context for Farol

Prepare the current session to supply factual PR context to a Farol workflow.
Apply this skill only within a Farol workflow or when explicitly requested for Farol.

## Prerequisites

Git, an authenticated GitHub CLI (`gh`), and Bash must be available. The
[checkout helper](scripts/ensure_pr_checkout.sh) is included with this skill.
Run it by its installed path while working inside the repository under review.

## Guardrails

- Collect facts and architecture, not findings.
- Do not assign severity, recommend approval, search for defects, or post comments.
- During context gathering, do not run tests, create commits, push, or make unrelated mutations.
- The only allowed repository mutations are fetches and the requested safe checkout/update of the PR branch.
- Never use force checkout, reset, stash, or discard local changes.
- Attribute existing review findings to their authors; do not present them as your conclusions.

## Workflow

1. Determine the PR selector from the request. Accept a number or URL. Ask for the selector if it cannot be established unambiguously from the request.
2. Read the repository's applicable agent instructions, then the smallest relevant module instructions or README files after the changed modules are known.
3. Announce that this skill is aligning the worktree and gathering context without performing a review.
4. Run:

   ```bash
   bash "<installed-skill-directory>/scripts/ensure_pr_checkout.sh" "<pr-selector>"
   ```

5. If the script reports `status=checked_out`, reread the applicable repository instructions from the checked-out branch before continuing.
6. If checkout is required and the worktree is dirty, stop. Report the dirty paths and ask the user to clean, commit, or move them. Never resolve this automatically.
7. If the script reports an aligned but dirty worktree, preserve it. Use committed objects (`git show`, `git diff <base>...<head>`) instead of working-tree file contents for PR code, and clearly disclose the local modifications.
8. Gather PR metadata with `gh pr view`, including:
   - title, body, author, URL, state, draft status, mergeability, review decision;
   - base/head refs and OIDs;
   - additions, deletions, changed files, commits, requested reviewers, and current checks.
9. Read the exact committed diff between the PR base and head. Separate production code, tests, and documentation so generated volume or large test additions do not obscure the implementation size.
10. Map the change:
   - user problem and promised behavior;
   - main execution/data flow;
   - contracts, state transitions, persistence, routes, and external dependencies;
   - affected modules and the most useful entry-point files;
   - rollout, migration, compatibility, or companion-PR requirements explicitly documented by the author.
11. Read existing review and issue comments only as historical context. Record whether threads were answered or superseded by later commits.
12. Follow material companion PR links when they define a coupled contract. Report their current state and checks without reviewing them.
13. If context gathering is the final requested outcome, return a concise context brief in the user's language. Explicitly state that:
    - no independent review conclusion was made;
    - no tests were run because this was a read-only context pass;
    - the session is ready for focused questions.

## Useful commands

Prefer exact PR objects over a possibly stale local base branch:

```bash
gh pr view <pr> --json number,title,body,author,url,state,isDraft,mergeable,reviewDecision,baseRefName,baseRefOid,headRefName,headRefOid,additions,deletions,changedFiles,commits,files,statusCheckRollup,latestReviews,reviewRequests
gh api --paginate repos/{owner}/{repo}/pulls/<pr>/comments
gh api --paginate repos/{owner}/{repo}/issues/<pr>/comments
git diff --stat <base-oid>...<head-oid>
git diff --numstat <base-oid>...<head-oid>
```

Use `rg` to locate module documentation, types, functions, routes, schemas, and design documents. Read only the smallest set needed to explain the change.

## Output shape

Keep the brief factual and easy to query later:

- PR status and scope;
- change purpose;
- execution or data flow;
- principal files and modules;
- coupled work, migrations, or rollout notes;
- attributed review history and current checks;
- confirmation that no review was performed.

## Use from another workflow

When another skill invokes this workflow, gather the same factual context
and make it available to that workflow.

Include:
- The repository, PR, and exact comparison refs and commits.
- The purpose documented by the author.
- The principal files and execution flow.
- Relevant requirements, decisions, and discussion.
- Any uncertainty or local checkout limitation.

Keep source attribution so the calling workflow can distinguish documented
decisions from interpretation.

Continue into the calling workflow after gathering context. Produce the
standalone context brief only when the user requested context gathering
as the final outcome.

The restriction against producing review findings applies to this context
gathering step. Subsequent work follows the scope the user authorized.
