# Installing Farol

The [README](../README.md#install) has the download and installation commands.
Packages contain the executable and documentation. The frontend is embedded in
the executable, so you do not need Node, Rust, or the source checkout to run it.

## Choose a package

On macOS, **Apple menu → About This Mac** shows whether the machine has an Apple
chip or an Intel processor. On Linux, `uname -m` must report `x86_64` for the
current package.

The first packages support macOS 15+ and Linux with glibc 2.35+ (tested on Ubuntu
22.04). Windows, Linux ARM64, and musl-based distributions such as Alpine are
not supported by these packages.

## Verify the download

Download the archive and `SHA256SUMS` from the same release. Keep just that one
archive in the folder where you run the checksum command from the README.
`--ignore-missing` skips the other platforms listed in the checksum file.

Continue only when your archive reports `OK`. If verification fails, download
both files again from the same release. A checksum verifies integrity, not
publisher identity.

## Keep Farol on your PATH

The installation places the executable in `~/.local/bin`. The README's `export`
command makes it available in the current terminal. To keep it available in new
terminals, add this line to `~/.zshrc` for zsh or `~/.bashrc` for bash:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Open a new terminal and run `farol --version`.

## macOS approval

The preview packages are not Developer ID signed or notarized. If macOS blocks
Farol, verify that you downloaded it from this repository's release page and
follow [Apple's instructions for software from an unknown developer](https://support.apple.com/guide/mac-help/mh40616/mac).
Farol does not change system security settings.

## Update

1. Download and verify the new package.
2. Run `farol servers` and stop each review you want to update with
   `farol servers stop PORT`, using its listed port.
3. Repeat the extraction and installation commands in the README.
4. Run `farol serve` from each repository to reopen its review.

There is no automatic updater. Running servers keep using the old executable
until they are restarted.

## Remove

Stop the running reviews, then remove the executable:

```bash
rm "$HOME/.local/bin/farol"
```

This leaves your configuration and worktree review data in place. The data lives
under each worktree's Git directory; removing a worktree removes its review data.
