# acil

[![Release](https://img.shields.io/github/v/release/a666hn/acil?include_prereleases)](https://github.com/a666hn/acil/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/a666hn/acil/release.yml?branch=main)](https://github.com/a666hn/acil/actions)
[![License](https://img.shields.io/github/license/a666hn/acil)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](#installation)
[![Jira](https://img.shields.io/badge/Jira-0052CC?logo=jira&logoColor=white)](https://www.atlassian.com/software/jira)
[![Confluence](https://img.shields.io/badge/Confluence-172B4D?logo=confluence&logoColor=white)](https://www.atlassian.com/software/confluence)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](#contributing)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg)](https://conventionalcommits.org)

> A fast, cross-platform CLI for managing Jira and Confluence with multi-account support.

## Why acil

Working across Jira and Confluence usually means a browser, a dozen tabs, and re-authenticating every time you switch Atlassian accounts. **acil** puts both behind one fast, scriptable command-line tool instead: switch between accounts instantly with profiles, script issue and page operations in shell pipelines or CI, or drop into a colored, interactive REPL for everyday triage. It ships as a single static binary — no runtime, no system dependencies — and behaves identically on macOS, Linux, and Windows.

## Table of Contents

- [Why acil](#why-acil)
- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Global Flags](#global-flags)
- [Command Reference](#command-reference)
  - [Jira](#jira)
  - [Confluence](#confluence)
  - [Profile](#profile)
  - [Self-Management](#self-management)
- [Interactive REPL](#interactive-repl)
- [Configuration](#configuration)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Multi-profile accounts** — Switch between multiple Atlassian accounts instantly
- **Jira management** — List, create, transition issues with JQL support
- **Confluence pages** — Search, list (tree view), pull/push pages as local markdown files
- **Markdown conversion** — Write Jira descriptions and Confluence pages in markdown; acil converts to Atlassian Document Format and Confluence storage format automatically, and back again on pull
- **Subtask support** — View issues with nested subtasks in tree format
- **Colored, interactive REPL** — Persistent shell with a colored prompt, command syntax highlighting, fish-style history hints, Tab-completion, and paged output
- **Colorized output** — Jira status/type and Confluence page type are color-coded at a glance; automatically disabled when piped or when `NO_COLOR` is set
- **Cross-platform** — Runs on macOS, Linux, and Windows with zero system dependencies
- **Fast** — Built in Rust with async HTTP (tokio + reqwest)
- **Self-updating** — `acil update` fetches and installs the latest release in place; `acil uninstall` removes it cleanly

## Installation

### Quick Install (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/a666hn/acil/main/install.sh | sh
```

### Specific Version

```bash
curl -fsSL https://raw.githubusercontent.com/a666hn/acil/main/install.sh | sh -s -- --version v0.1.5
```

Use `--bin-dir <path>` to install somewhere other than the default (`/usr/local/bin`, falling back to `~/.local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/a666hn/acil/main/install.sh | sh -s -- --bin-dir ~/bin
```

### Windows

`install.sh` also works on Windows from **Git Bash, MSYS2, or WSL** (it detects and packages for `x86_64-pc-windows-msvc` automatically):

```bash
curl -fsSL https://raw.githubusercontent.com/a666hn/acil/main/install.sh | sh
```

From a native `cmd.exe` or PowerShell prompt (no `sh` available), use Manual Download or build from source below instead.

### Build from Source

`acil` isn't published to crates.io yet, so `cargo install acil` won't work. Build it directly from the repository instead:

```bash
git clone https://github.com/a666hn/acil.git
cd acil
cargo install --path .
```

Or build a release binary without installing it onto your `PATH`:

```bash
cargo build --release
# binary at target/release/acil
```

### Manual Download

Download the latest binary for your platform from [GitHub Releases](https://github.com/a666hn/acil/releases):

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `acil-*-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `acil-*-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `acil-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux (ARM64) | `acil-*-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `acil-*-x86_64-pc-windows-msvc.zip` |

## Quick Start

### 1. Login to Atlassian

Start the interactive REPL and add your account:

```bash
acil
acil()> login --name work --url https://your-domain.atlassian.net --email you@company.com
# Enter Jira and Confluence API tokens when prompted
```

Jira and Confluence use separate API tokens even on the same Atlassian site — acil prompts for both. Tokens can be generated at: https://id.atlassian.com/manage-profile/security/api-tokens

### 2. Use Commands

```bash
# List your Jira issues
acil jira list

# List Confluence pages as a tree
acil confluence pages --space PROJ --tree

# Pull a Confluence page, edit locally, push back
acil confluence pull 12345678
vim 12345678.md
acil confluence push 12345678.md
```

### 3. Interactive REPL

```bash
acil
# Colored prompt, Tab-completion, and history hints
acil(work)> jira list --limit 5
acil(work)> confluence pages --space PROJ --tree
acil(work)> profile switch personal
acil(personal)> exit
```

## Global Flags

Available on every one-shot command (`acil <flags> <command> ...`):

| Flag | Description |
|---|---|
| `-p, --profile <name>` | Override the active profile for this command only |
| `-v, --verbose` | Log outgoing API requests (and the raw Jira response) to stderr |
| `NO_COLOR=1` (env var) | Disable all colored output. Color is also auto-disabled whenever stdout isn't a terminal, e.g. when piping to a file |

These are one-shot CLI flags, not REPL commands — inside the REPL, switch profiles with `profile switch <name>` instead, and `--verbose` isn't available (use one-shot CLI mode for verbose logging).

## Command Reference

### Jira

```bash
acil jira list                                                        # All issues, any status/assignee, most recently updated first
acil jira list --status "In Progress"                                 # Filter by status
acil jira list --assigned you@company.com                             # Filter by assignee
acil jira list --query "project = PROJ AND status = 'In Progress'"    # Custom JQL
acil jira list --subtasks --tree                                      # With subtask tree
acil jira get PROJ-123                                                # Get issue details
acil jira create --project PROJ --summary "Fix bug" --priority High   # Create issue
acil jira transition PROJ-123 --status "In Progress"                  # Transition issue
```

**`jira list`**

With no flags, `jira list` shows the most recently updated issues across the whole instance — no status or assignee filter. Use `--assigned`/`--status` to narrow it down, or `--query` for full custom JQL.

| Flag | Description |
|---|---|
| `-q, --query <jql>` | Custom JQL query — overrides `--assigned`/`--status` entirely |
| `-a, --assigned <email>` | Filter by assignee email |
| `-s, --status <name>` | Filter by status, e.g. "To Do", "In Progress", "Done", "In Review", "On Hold" |
| `-m, --max <n>` | Total results to return (default: 10) — automatically fetches as many pages as needed |
| `-l, --limit <n>` | Limit displayed results |
| `--subtasks` | Include subtasks in output |
| `--tree` | Show subtasks grouped under parents (requires `--subtasks`) |

**`jira create`**

| Flag | Description |
|---|---|
| `--project <key>` | Project key (required) |
| `-s, --summary <text>` | Issue summary (required) |
| `-i, --issue-type <type>` | Issue type, e.g. Bug, Story, Task, Sub-task (default: Task) |
| `--priority <p>` | Priority, e.g. Highest, High, Medium, Low, Lowest |
| `-l, --labels <l>` | Comma-separated labels |
| `-a, --assignee <a>` | Assignee (username or email) |
| `--parent <key>` | Parent issue key, for creating subtasks |
| `-d, --description <md>` | Description, written in markdown and converted to ADF |

**`jira transition <key>`**

| Flag | Description |
|---|---|
| `-s, --status <status>` | Target status name (required) |

**`jira get <key>`** takes no flags beyond the [global ones](#global-flags).

### Confluence

```bash
acil confluence search "meeting notes"                    # Search pages
acil confluence pages --space PROJ                        # List pages
acil confluence pages --space PROJ --tree                 # Tree view
acil confluence get 12345678                               # Get page content
acil confluence pull 12345678                              # Download as markdown
acil confluence push 12345678.md                           # Push changes back
acil confluence create --space PROJ --title "New Page"     # Create page
acil confluence create --space PROJ --title "Page" --file template.md  # From file
acil confluence update 12345678 --title "New Title"       # Update page
```

**`confluence search <query>`**

| Flag | Description |
|---|---|
| `-m, --max <n>` | Max results (default: 10) |
| `-l, --limit <n>` | Limit displayed results |

**`confluence pages`**

| Flag | Description |
|---|---|
| `-s, --space <key>` | Space key, e.g. PROJ (required) |
| `-t, --tree` | Show pages as an indented tree |
| `-m, --max <n>` | Max results per API page (default: 250) |
| `-l, --limit <n>` | Limit displayed results |

**`confluence pull <id>`**

| Flag | Description |
|---|---|
| `-o, --output <dir>` | Output directory (default: current directory) |

**`confluence create`**

| Flag | Description |
|---|---|
| `-s, --space <key>` | Space key (required) |
| `-t, --title <text>` | Page title (required) |
| `-f, --file <path>` | Create from a local markdown file |
| `--parent <id>` | Parent page ID |

**`confluence update <id>`**

| Flag | Description |
|---|---|
| `-t, --title <text>` | New title |
| `-b, --body <text>` | New content (body) |

**`confluence get <id>`** and **`confluence push <file>`** take no flags beyond the [global ones](#global-flags).

### Profile

```bash
acil profile list           # List all profiles
acil profile current        # Show active profile
acil profile rm personal    # Remove a profile
```

**REPL-only commands** — `login` and `profile switch` are interactive/auth operations and only run inside the REPL, not as one-shot CLI commands:

```bash
acil(work)> login --name personal --url https://personal.atlassian.net --email me@personal.com
acil(work)> profile switch personal
acil(personal)>
```

### Self-Management

```bash
acil update                 # Check for and install the latest release
acil update --yes           # Same, without the confirmation prompt
acil uninstall               # Remove the acil binary (asks for confirmation)
acil uninstall --yes         # Remove without prompting
acil uninstall --purge-config  # Also remove ~/.config/acil (profiles, API tokens, history)
```

**`update`**

| Flag | Description |
|---|---|
| `-y, --yes` | Skip the confirmation prompt |

Downloads and installs the latest GitHub release for your platform, replacing the currently running binary in place — works regardless of whether you installed via `install.sh`, `cargo install --path .`, or a manual download. If the binary lives in a directory you don't have write access to, re-run with `sudo` or use `install.sh` instead.

**`uninstall`**

| Flag | Description |
|---|---|
| `-y, --yes` | Skip the confirmation prompt |
| `--purge-config` | Also delete `~/.config/acil` (profiles, API tokens, and history) |

Removes only the binary by default — your profiles and API tokens in `~/.config/acil` are left untouched unless you explicitly pass `--purge-config`.

`update` and `uninstall` are one-shot CLI commands only; they aren't available inside the REPL.

## Interactive REPL

Running `acil` with no arguments starts a persistent REPL built on [reedline](https://github.com/nushell/reedline) (the same line editor that powers Nushell):

- **Colored prompt** — shows the active profile, e.g. `acil(work)>`
- **Syntax highlighting** — known commands are highlighted as you type
- **History hints** — start retyping a previous command and a dim inline suggestion completes it, fish-shell style
- **Tab-completion** — press <kbd>Tab</kbd> after a partial command to open a completion menu
- **Persistent history** — stored at `~/.config/acil/history.txt`, restored across sessions

REPL commands mirror the one-shot CLI subcommands exactly (`jira list`, `confluence pages --space PROJ --tree`, etc.), plus the two REPL-only commands above. <kbd>Ctrl-C</kbd> and <kbd>Ctrl-D</kbd> both exit the REPL. Long output is paged 20 lines at a time — press Enter to continue or `q` to stop.

## Configuration

Config location: `~/.config/acil/config.yaml`

```yaml
active_profile: work
profiles:
  work:
    jira:
      base_url: https://work.atlassian.net
      email: user@work.com
      api_token: <token>
    confluence:
      base_url: https://work.atlassian.net
      email: user@work.com
      api_token: <token>
  personal:
    jira:
      base_url: https://personal.atlassian.net
      email: user@personal.com
      api_token: <token>
    confluence:
      base_url: https://personal.atlassian.net
      email: user@personal.com
      api_token: <token>
```

REPL command history is stored separately at `~/.config/acil/history.txt`.

## Development

### Build

```bash
cargo build              # debug build
cargo build --release    # optimized release binary
```

### Test

```bash
cargo test               # run all tests
cargo test <module>      # run single module (e.g., cargo test jira_adf)
```

### Lint & Format

```bash
cargo clippy             # lint
cargo fmt                # auto-format
cargo fmt --check        # check format (CI uses this)
```

### Verification Order

Run before committing:

```bash
cargo fmt --check && cargo clippy && cargo test
```

### Release

1. Update version in `Cargo.toml`
2. Commit: `git commit -am "chore: bump version to X.Y.Z"`
3. Tag: `git tag vX.Y.Z`
4. Push: `git push --tags`
5. GitHub Actions builds binaries for all platforms and publishes the release automatically

### Cross-Compilation Targets

| Target | Command |
|---|---|
| macOS Intel | `cargo build --release --target x86_64-apple-darwin` |
| macOS Apple Silicon | `cargo build --release --target aarch64-apple-darwin` |
| Linux x86_64 | `cross build --release --target x86_64-unknown-linux-musl` |
| Linux ARM64 | `cross build --release --target aarch64-unknown-linux-gnu` |
| Windows | `cargo build --release --target x86_64-pc-windows-msvc` |

## Contributing

Feature specs live in [`spec/`](spec/) — read the relevant one before implementing a change; each describes the intended design and files to touch. [`AGENTS.md`](AGENTS.md) covers project conventions in more depth. In short:

- Commits follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`)
- Run `cargo fmt --check && cargo clippy && cargo test` before committing
- clap uses derive macros throughout, not the builder pattern
- REPL commands mirror one-shot subcommands exactly — keep them in sync when adding new flags
- No secrets in code or commits

## License

[MIT](LICENSE)
