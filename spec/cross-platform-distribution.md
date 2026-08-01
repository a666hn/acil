# Spec: Cross-Platform Distribution

**Status**: Implemented
**Created**: 2025-08-01

## Goal

Make `acil` (renamed from `atlas-cli`) available as a cross-platform CLI tool. Users on macOS, Linux, and Windows can install with one command.

## Binary Rename

Rename binary from `atlas-cli` to `acil`:
- Update `Cargo.toml`: `name = "acil"`
- Update all references in source code, AGENTS.md, REPL prompt, config paths
- Config path: `~/.config/acil/config.yaml`
- History path: `~/.config/acil/history.txt`

## Distribution Strategy

| Method | Command | Platforms |
|---|---|---|
| Shell installer | `curl -fsSL .../install.sh \| sh` | macOS, Linux |
| cargo install | `cargo install acil` | All |
| GitHub Releases | Download from releases page | All |

## Step 1: Fix `Cargo.toml`

```toml
[package]
name = "acil"
version = "0.1.0"
edition = "2024"
description = "CLI for managing Jira and Confluence with multi-profile support"
license = "MIT"
repository = "https://github.com/digdaya-ai/acil"
keywords = ["jira", "confluence", "atlassian", "cli"]

[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

## Step 2: GitHub Actions — `.github/workflows/release.yml`

Trigger: push tag `v*`

### Build Matrix

| Target | Runner | Cross? | Output |
|---|---|---|---|
| `x86_64-unknown-linux-musl` | ubuntu-latest | `cross` | Static binary |
| `aarch64-unknown-linux-gnu` | ubuntu-latest | `cross` | ARM64 Linux |
| `x86_64-apple-darwin` | macos-13 | native | Intel Mac |
| `aarch64-apple-darwin` | macos-latest | native | Apple Silicon |
| `x86_64-pc-windows-msvc` | windows-latest | native | Windows |

### Workflow

1. Create draft GitHub Release
2. Build binary for each target
3. Package: `.tar.gz` (Unix) or `.zip` (Windows)
4. Upload to Release
5. Publish Release

### Release workflow

```bash
# Update version in Cargo.toml
git commit -am "chore: bump version to 0.2.0"
git tag v0.2.0
git push --tags
# GitHub builds everything automatically
```

## Step 3: Shell Installer — `install.sh`

User command:
```bash
curl -fsSL https://raw.githubusercontent.com/digdaya-ai/acil/main/install.sh | sh
```

### What it does

1. Detect OS: `uname -s` → `linux`, `darwin`, `windows`
2. Detect arch: `uname -m` → `x86_64`, `aarch64`
3. Map to target triple
4. Download from GitHub Releases (latest)
5. Extract to `/usr/local/bin` or `~/.local/bin`
6. Verify: `acil --version`

### Features

- `--version` flag to install specific version
- `--bin-dir` flag to choose install location
- Auto-detects if `sudo` is needed
- Falls back to `~/.local/bin` if no write access
- Supports `curl` and `wget`

## Step 4: Update Source References

All references to `atlas-cli` → `acil`:
- `src/main.rs` — clap name
- `src/repl.rs` — prompt, help text
- `src/config.rs` — config directory name
- `AGENTS.md` — all references
- `opencode.json` — if needed

## Files to Create/Modify

| File | Action | Purpose |
|---|---|---|
| `Cargo.toml` | Edit | Rename to `acil`, switch to `rustls-tls`, add metadata |
| `src/main.rs` | Edit | Update clap name |
| `src/repl.rs` | Edit | Update prompt and help text |
| `src/config.rs` | Edit | Update config directory name |
| `.github/workflows/release.yml` | New | CI/CD build pipeline |
| `install.sh` | New | Shell installer script |
| `README.md` | New | Installation + usage docs |
| `AGENTS.md` | Edit | Update all references |

## Verification

1. `cargo fmt --check`
2. `cargo clippy`
3. `cargo test`
4. `cargo build --release` — verify binary name is `acil`
5. Test install script locally
