# Release packages

The `release packages` workflow builds native archives for macOS Apple Silicon,
macOS Intel, and Linux x86_64. Each archive contains the executable and README;
the frontend is embedded in the executable.

Pull requests and manual workflow runs upload build artifacts only. A pushed
`v*` tag publishes a GitHub prerelease after every package passes its smoke
check. No stable-release promotion or signing is automated here.

## Inspect packages before publishing

1. Open the `release packages` run for the pull request.
2. Require all three package jobs to succeed.
3. Download the matching `farol-TARGET` workflow artifact. It contains a
   `.tar.gz` and a `.sha256` file.
4. Check its checksum and extract it. Run the executable from outside the
   source checkout and confirm its version.

The automated smoke check performs this extraction in a temporary directory,
using an isolated Git repository, configuration, and server registry. It checks
CLI execution, map creation, the review API, file content, embedded HTML/JS/CSS,
server reuse, and shutdown. It never reads the developer's GitHub credential or
stops their running reviews.

The deployment target for both macOS archives is macOS 15. The Linux build uses
Ubuntu 22.04 and targets glibc 2.35 or newer. Each package executes on a runner
of its own architecture; these checks are not cross-compilation alone.

## Build one package locally

Use the pinned Rust and Node toolchains and Just 1.57.0. Packaging and smoke
checks also use Bash, Git, tar, curl, jq, and `shasum`. These are build-time
requirements; release users do not need the frontend or Rust toolchains.

On the matching machine, run one of:

```bash
just package aarch64-apple-darwin
just package x86_64-apple-darwin
just package x86_64-unknown-linux-gnu
```

For macOS packages intended for distribution, set
`MACOSX_DEPLOYMENT_TARGET=15.0` before building, as the workflow does. The package
name uses the version reported by the built executable:

```bash
just smoke-package dist/farol-vVERSION-TARGET.tar.gz
```

`dist/` is ignored by Git. The recipes use `npm ci` and Cargo's `--locked` mode;
a failed dependency resolution must be fixed in the lockfile, not silently
replaced during release packaging.

## Publish a prerelease

Publishing is a separate maintainer action after the packaging PR is merged.
The first version has not been selected or tagged by this change.

1. Choose the version, for example `0.1.0-alpha.1`, with the maintainer.
2. Update the root package version in `Cargo.toml` and its root entry in
   `Cargo.lock`. Merge that version change after checks pass.
3. Confirm the license/publication checklist is ready. This workflow does not
   choose a license or change the repository's visibility.
4. Create and push a tag whose name is exactly `v` followed by the Cargo version.
   A mismatch fails packaging and prevents publication.
5. The workflow builds and tests all packages, combines their checksums into
   `SHA256SUMS`, verifies them, and publishes the archives as a prerelease.
6. Inspect the release page and download a package to verify the published assets.

Do not point a prerelease tag at a binary that reports a different version.
macOS archives are not Developer ID signed or notarized; document this condition
when announcing the release. Checksums are not a replacement for signing.
