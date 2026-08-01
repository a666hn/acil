# acil

[![Release](https://img.shields.io/github/v/release/a666hn/acil?include_prereleases)](https://github.com/a666hn/acil/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/a666hn/acil/release.yml?branch=main)](https://github.com/a666hn/acil/actions)
[![License](https://img.shields.io/github/license/a666hn/acil)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](#installation)

> A fast, cross-platform CLI for managing Jira and Confluence with multi-account support.

## Features

- **Multi-profile accounts** — Switch between multiple Atlassian accounts instantly
- **Jira management** — List, create, transition issues with JQL support
- **Confluence pages** — Search, list (tree view), pull/push pages as local markdown files
- **Markdown to ADF** — Write descriptions in markdown, auto-converts to Atlassian Document Format
- **Subtask support** — View issues with nested subtasks in tree format
- **Interactive REPL** — Persistent shell with command history and paged output
- **Cross-platform** — Runs on macOS, Linux, and Windows with zero system dependencies
- **Fast** — Built in Rust with async HTTP (tokio + reqwest)

## Installation

### Quick Install (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/a666hn/acil/main/install.sh | sh
```

### Specific Version

```bash
curl -fsSL https://raw.githubusercontent.com/a666hn/acil/main/install.sh | sh -s -- --version v0.1.0
```

### Cargo (any platform)

```bash
cargo install acil
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

API tokens can be generated at: https://id.atlassian.com/manage-profile/security/api-tokens

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
# REPL mode with command history and paged output
acil(work)> jira list --limit 5
acil(work)> confluence pages --space PROJ --tree
acil(work)> profile switch personal
acil(personal)> exit
```

## Commands

### Jira

```bash
acil jira list                                                        # List your issues
acil jira list --query "project = PROJ AND status = 'In Progress'"    # Custom JQL
acil jira list --subtasks --tree                                      # With subtask tree
acil jira get PROJ-123                                                # Get issue details
acil jira create --project PROJ --summary "Fix bug" --priority High   # Create issue
acil jira transition PROJ-123 --status "In Progress"                  # Transition issue
```

| Flag | Description |
|---|---|
| `--query <jql>` | Custom JQL query |
| `--max <n>` | Max results from API (default: 10) |
| `--limit <n>` | Limit displayed results |
| `--subtasks` | Include subtasks in output |
| `--tree` | Show subtasks grouped under parents |

### Confluence

```bash
acil confluence search "meeting notes"                    # Search pages
acil confluence pages --space PROJ                        # List pages
acil confluence pages --space PROJ --tree                 # Tree view
acil confluence get 12345678                              # Get page content
acil confluence pull 12345678                             # Download as markdown
acil confluence push 12345678.md                          # Push changes back
acil confluence create --space PROJ --title "New Page"    # Create page
acil confluence create --space PROJ --title "Page" --file template.md  # From file
```

| Flag | Description |
|---|---|
| `--space <key>` | Space key (e.g., PROJ) |
| `--tree` | Show pages as indented tree |
| `--limit <n>` | Limit displayed results |
| `--file <path>` | Create from local markdown file |
| `--output <dir>` | Output directory for pull |

### Profile

```bash
acil profile list           # List all profiles
acil profile current        # Show active profile
acil profile rm personal    # Remove a profile
```

**REPL-only commands:**

```bash
acil(work)> login --name personal --url https://personal.atlassian.net --email me@personal.com
acil(work)> profile switch personal
acil(personal)>
```

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
5. GitHub Actions builds binaries for all platforms automatically

### Cross-Compilation Targets

| Target | Command |
|---|---|
| macOS Intel | `cargo build --release --target x86_64-apple-darwin` |
| macOS Apple Silicon | `cargo build --release --target aarch64-apple-darwin` |
| Linux x86_64 | `cross build --release --target x86_64-unknown-linux-musl` |
| Linux ARM64 | `cross build --release --target aarch64-unknown-linux-gnu` |
| Windows | `cargo build --release --target x86_64-pc-windows-msvc` |

## License

[MIT](LICENSE)
