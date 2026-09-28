---
name: farol-release
description: Prepare a Farol release version PR, or publish an explicitly authorized release and verify its artifacts. Use when asked to prepare, cut, or publish a Farol release.
---

# Release Farol

Use Git and the authenticated `gh` CLI from this repository. Read the
[release guide](../../../docs/releases.md) and `.github/workflows/release.yml`
before acting. They own the packaging commands, supported platforms, and
publication behavior; use the existing workflow for building and publishing.

## Prepare the version

- Inspect the working tree, `origin/main`, current Cargo version, existing tags
  and releases, and any version PR already in progress. Fetch the remote state
  before selecting the release commit.
- Use the version the user specified or already approved. If none was chosen,
  ask for it before changing version files. Do not invent a version.
- Prepare the version change on a dedicated branch from current `origin/main`.
  Update the root package version in `Cargo.toml` and its matching entry in
  `Cargo.lock`, keeping dependency versions unchanged.
- Draft the release notes (see below) from the commits and PRs since the last
  tag, and show the draft to the user for approval.
- Run `just check`, commit, and open or update the version PR. Require the
  normal CI to pass; native packages run after the merge to `main`. Report the
  PR link. A request to prepare a release ends here, without a tag push.

## Publish an authorized version

- Verify that the version change is merged into `main`, checks passed, and the
  publication prerequisites in the release guide are satisfied. Do not merge
  the version PR unless the user also authorized that action.
- Resolve the intended release commit on `origin/main`. Read its Cargo manifest
  and lockfile, and require the tag to be exactly `v` plus that package version.
- Require all three native package jobs for that commit on `main` to pass.
- Pushing this tag triggers publication. Do it only when the user explicitly
  authorized publishing this release. Approval of the version PR or its merge
  is not publication approval. Reuse explicit publication authorization already
  given for this release rather than asking again.
- If that authorization is missing, finish preparation and present the repository,
  version, tag, commit SHA, and check/artifact results for approval.
- Once authorized, create an annotated tag at that verified commit and push only
  that tag to `origin`. If the tag or release already exists, inspect and report
  its state instead of recreating it. Never force-update or delete release tags.

## Write the release notes

The workflow publishes with a fixed placeholder, so the notes are always written
by hand. Replace the placeholder with the approved draft once the release exists:
`gh release edit vX.Y.Z --notes-file notes.md`.

Use exactly this shape, keeping only the sections that have entries:

```markdown
### New

- The progress count in the top bar jumps to the next unread file, like `n`.

### Improved

- Read and unread files are easy to tell apart in the sidebar.

### Fixed

- A file reopens when a change takes its read mark off.

**Full changelog:** https://github.com/asnunes/farol/compare/vPREV...vX.Y.Z
```

- One line per change, in the user's terms: what they can now do or no longer
  run into. Group several commits for one visible change into a single line.
- Base each line on the merged PRs and commits since the previous tag. Never
  list a change that did not ship.

Do not include:

- An install, download, platform or requirements section. The asset list right
  below the notes already shows the packages.
- `SHA256SUMS`, checksums, update steps, or the signing notice.
- An intro paragraph or summary line.
- An explanation of what the old bug was, how it was found, or how it was fixed.
- Refactors, tests, CI, docs and other changes a user cannot see.
- PR numbers, commit hashes, file names or internal type names.

## Verify the published artifacts

- Follow the `release packages` run triggered by that tag. If it fails, inspect
  the logs and report the failed step before attempting another publication.
- Verify the expected tag and release type: a version with a prerelease suffix
  must be a prerelease; a version without one must be stable and marked latest.
  Require all three platform archives plus `SHA256SUMS`.
- Download those assets into a temporary directory and verify every checksum.
  Use the workflow's native smoke results as evidence that each package runs on
  its target platform.
- Confirm the release notes are the approved draft, not the placeholder.
- Report the release URL, tag, commit, and verification results. Do not announce
  the release in other channels unless the user requested that separately.
