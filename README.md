<p align="center">
  <img src="docs/logo.png" alt="farol" width="420">
</p>

<p align="center">
  A walkthrough, not a diff: the author's order, and the reasons behind it.
</p>

Farol is a local code-review app. It turns a branch diff into a guided walkthrough:
**files ordered to help you understand the feature, not alphabetically by path.**
The map puts the core implementation first and builds on it, with explanations
from the coding session that made the change. Read the code, leave comments,
and send the finished review to GitHub.

![A Farol review with reading-order blocks, implementation context, skim markers, and the code diff.](docs/review.jpg)

## Install

Download **one archive** and `SHA256SUMS` from the same
[release](https://github.com/asnunes/farol/releases) into an empty folder.

| Your machine | Choose the archive ending in |
|---|---|
| Mac with Apple Silicon, macOS 15+ | `aarch64-apple-darwin.tar.gz` |
| Mac with Intel, macOS 15+ | `x86_64-apple-darwin.tar.gz` |
| Linux x86_64, Ubuntu 22.04+ or glibc 2.35+ | `x86_64-unknown-linux-gnu.tar.gz` |

In that folder, verify the download:

- **macOS:** `shasum -a 256 --check SHA256SUMS --ignore-missing`
- **Linux:** `sha256sum --check SHA256SUMS --ignore-missing`

Once it reports `OK`, install:

```bash
tar -xzf farol-*.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 ./farol "$HOME/.local/bin/farol"
export PATH="$HOME/.local/bin:$PATH"
farol --version
```

You need Git and a browser. macOS may require [approval to run the unsigned
binary](docs/install.md#macos-approval). For updates, removal, or PATH setup,
see the [installation guide](docs/install.md).

No release available yet? [Build from source](docs/development.md).

## Install the agent skills

In the project you want to review, install the complete skill set:

```bash
npx skills add https://github.com/asnunes/farol/tree/main/skills --skill '*'
```

The installer handles agent selection; add `--agent codex` or another supported
agent to choose explicitly. See [skills and workflows](docs/skills.md).

## Use cases

### Review AI-generated code

Before accepting the change, ask the agent **in the session that implemented
it** to prepare the walkthrough:

```text
Use farol-maintain-map to prepare a map of this change for me to review.
```

That session can explain decisions that the code alone cannot: why an approach
was chosen, which tradeoffs were accepted, and what was deliberately left out.
The map gives you context to judge the implementation yourself.

Open it from the repository:

```bash
farol serve
```

On macOS the browser opens automatically. On Linux, open the printed URL.
Farol needs a map before it can serve a review. You can also
[write one by hand](docs/usage.md#write-a-map).

The same flow works for reviewing your own code before opening a PR, including
code written without AI; provide the implementation context to the agent.

### Open a PR for someone else to review

Give the reviewer the reading order and the reasons behind your change, even
though they were not in your implementation session. Prepare or update the map
with `farol-maintain-map`, then export it once the code is committed:

```text
Use farol-share-map to export this map for the reviewer of my PR.
```

The skill reports the full path of a JSON file. Attach that file to the PR or
send it directly to the reviewer. The JSON carries the map and its explanations;
the reviewer checks out the PR's code separately on their own machine. Your
local comments and reading progress are not included.

### Review someone else's PR

You can use Farol whether or not the author prepared a map.

**With an author's map:** download the JSON they attached or sent, then ask:

```text
Use farol-import-map for PR <URL> with the map at <local JSON path>.
```

The skill prepares the PR checkout, imports the author's explanations, and
opens the review on your machine.

**Without a map:** ask your agent to build one from the available PR context:

```text
Use farol-map-from-review for PR <URL>, which has no map.
```

The skill organizes the changes using the code and documented PR context. It
cannot recover undocumented decisions from the author's session.

In either case, read the code and leave your own comments. Use **Send review**
in the app, or ask `farol-publish-review` to prepare your existing review for
approval and publication. Your comments and reading progress stay local until
you choose to publish.

## Review

### Read in the order the feature makes sense

The map orders blocks and files by their importance to understanding the change.
Start with the core implementation, then follow the parts that depend on it;
when one file needs context from another, that context comes first. Files from
different folders can sit together in the same block, with the implementation
decisions beside the code they explain. The sidebar and diff follow this reading
order instead of an alphabetical file tree.

Mark files as read as you go; your progress stays local and survives new commits
until a file changes.

### Compare side by side

Switch between unified and split views to read changes in the layout you prefer.
Split view puts the old and new code next to each other.

![Split view showing the old and new versions of two configuration files.](docs/review-split.jpg)

### Leave comments on the code

Click `+` or drag across line numbers to comment on a line or range. Comments
stay local until you use **Send review** to publish them to GitHub; the page
guides you through authorizing the destination host.

![A local example comment beside the changed line and its implementation context.](docs/review-comments.jpg)

### Know what you can skim

The map marks supporting files as **skim** when they can be skipped without
missing the core implementation. Each carries a reason, so you can decide
whether to open it anyway.

![Skim markers in the sidebar and reasons above the supporting configuration changes.](docs/review-skim.jpg)

Press `?` for keyboard shortcuts. Use `farol servers` to list open reviews.

## License

Farol is available under the [MIT License](LICENSE).

[CLI and behavior guide](docs/usage.md) · [Development](docs/development.md) ·
[Preparing a release](docs/releases.md)
