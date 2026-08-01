# AGENTS.md

## Project
acil — Rust CLI for managing Jira and Confluence. Supports multiple Atlassian accounts via profiles, one-shot commands, and interactive REPL mode.

## Tech Stack
- Rust (stable toolchain)
- Build: cargo
- CLI: clap (derive macros)
- REPL: rustyline
- HTTP: reqwest + tokio (rustls-tls, no OpenSSL dependency)
- Serialization: serde, serde_yaml, serde_json
- Errors: thiserror (library), anyhow (binary)
- Config: ~/.config/acil/config.yaml

## Commands
cargo build              # compile
cargo build --release    # optimized binary
cargo test               # all tests
cargo test <module>      # single module
cargo clippy             # lint
cargo fmt --check        # format check (run before commit)
cargo fmt                # auto-format
cargo run -- <subcmd>    # one-shot execution (REPL is default with no args)

## Verification Order (run before committing)
1. cargo fmt --check
2. cargo clippy
3. cargo test

## Project Structure
src/
  main.rs           # entrypoint, clap arg parsing, login, REPL dispatch
  commands/
    mod.rs
    jira.rs          # jira subcommands (list, get, create, transition)
    confluence.rs    # confluence subcommands (search, pages, get, pull, push, create, update)
    profile.rs       # profile management (list, switch, current, rm)
  repl.rs            # REPL loop (rustyline), command routing with profile support
  config.rs          # multi-profile config load/save, profile operations
  client.rs          # reqwest HTTP wrapper, auth headers
  error.rs           # thiserror types
  output.rs          # table/JSON/tree formatting
  confluence_convert.rs  # markdown ↔ Confluence storage format
  confluence_meta.rs     # page metadata, frontmatter parse/serialize
  jira_adf.rs            # markdown → Atlassian Document Format (ADF)

## Multi-Profile Config (~/.config/acil/config.yaml)
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

## CLI Usage
acil                                                          # interactive mode (REPL)
acil --profile <name> jira list                               # override profile per command
acil profile list                                             # list all profiles
acil profile current                                          # show active profile
acil profile rm <name>                                        # remove a profile

## REPL-Only Commands
login and profile switch are only available in REPL mode:
acil(login --name <profile> --url <url> --email <email>      # add profile (prompts for token)
acil(profile switch <name>                                    # switch active profile

## Specs
Feature specs live in `spec/`. Read the relevant spec before implementing a feature:
- `spec/confluence-pull-push.md` — local edit workflow (pull/push/create-from-file)
- `spec/confluence-pages.md` — hierarchical page listing with tree view
- `spec/pagination-pager.md` — CLI `--limit` flag + REPL interactive pager
- `spec/repl-only-auth.md` — REPL-only login and profile switch
- `spec/jira-list-enhanced.md` — enhanced jira list with type, assignee, URL, subtasks
- `spec/jira-create-enhanced.md` — enhanced jira create with priority, labels, assignee, parent, description (ADF)
- `spec/cross-platform-distribution.md` — cross-platform build and installation

## Conventions
- Commits: Conventional Commits (feat:, fix:, chore:, docs:, refactor:, test:)
- clap derive macros, not builder pattern
- REPL commands mirror subcommands (e.g., `jira list` in REPL = `acil jira list`)
- REPL prompt shows active profile: `acil(work)> `
- Errors: thiserror for library error enums, anyhow in main
- Async: tokio runtime everywhere
- No secrets in code or commits
