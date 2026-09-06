---
name: farol-release
description: Prepare a Farol release version PR, or publish an explicitly authorized prerelease and verify its artifacts. Use when asked to prepare, cut, or publish a Farol release.
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
  ask for it before changing version files. Do not invent the first alpha number.
- Prepare the version change on a dedicated branch from current `origin/main`.
  Update the root package version in `Cargo.toml` and its matching entry in
  `Cargo.lock`, keeping dependency versions unchanged.
- Run `just check`, commit, and open or update the version PR. Require both the
  normal CI and all three native package jobs to pass. Report the PR and artifact
  links. A request to prepare a release ends here, without a tag push.

## Publish an authorized version

- Verify that the version change is merged into `main`, checks passed, and the
  publication prerequisites in the release guide are satisfied. Do not merge
  the version PR unless the user also authorized that action.
- Resolve the intended release commit on `origin/main`. Read its Cargo manifest
  and lockfile, and require the tag to be exactly `v` plus that package version.
- Pushing this tag triggers publication. Do it only when the user explicitly
  authorized publishing this release. Approval of the version PR or its merge
  is not publication approval. Reuse explicit publication authorization already
  given for this release rather than asking again.
- If that authorization is missing, finish preparation and present the repository,
  version, tag, commit SHA, and check/artifact results for approval.
- Once authorized, create an annotated tag at that verified commit and push only
  that tag to `origin`. If the tag or release already exists, inspect and report
  its state instead of recreating it. Never force-update or delete release tags.

## Verify the published artifacts

- Follow the `release packages` run triggered by that tag. If it fails, inspect
  the logs and report the failed step before attempting another publication.
- Verify that the GitHub release is a prerelease for the expected tag and contains
  all three platform archives plus `SHA256SUMS`.
- Download those assets into a temporary directory and verify every checksum.
  Use the workflow's native smoke results as evidence that each package runs on
  its target platform.
- Report the release URL, tag, commit, and verification results. Do not announce
  the release in other channels unless the user requested that separately.
