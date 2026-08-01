# Spec: Pagination for CLI + Interactive Pager for REPL

**Status**: Implemented
**Created**: 2025-07-31

## Goal

Two pagination strategies based on context:
- **CLI mode**: `--limit` flag to cap displayed results
- **REPL mode**: interactive pager (press Enter to continue, q to quit)

```bash
# CLI: show first 20 results
atlas-cli confluence pages --space PROJ --limit 20
atlas-cli confluence search "docs" --limit 10

# REPL: interactive pager
atlas-cli(work)> confluence pages --space PROJ
Pages in PROJ — 50 pages
──────────────────────────
Getting Started (123456)
├─ Installation (234567)
└─ Configuration (345678)
... (17 more lines)

--- Press Enter to continue, q to quit ---
```

## Core Refactor: Return structured output

Current `execute()` functions print directly to stdout. Refactor to return `CommandOutput` so the caller (CLI or REPL) controls display.

```rust
pub enum CommandOutput {
    Lines(Vec<String>),   // table rows, tree lines, etc.
    Empty,                // no output (push, create confirmations)
}
```

- **CLI**: calls `output.print_all()` — dumps all lines immediately
- **REPL**: calls `paged_print(lines, page_size)` — pages with "Press Enter" prompt

## Commands Affected

| Command | CLI behavior | REPL behavior |
|---|---|---|
| `confluence pages --space <key> [--tree] [--limit N]` | Show all or first N | Paged |
| `confluence search <query> [--max N] [--limit N]` | Show all or first N | Paged |
| `confluence get <id>` | Print JSON (no change) | Print JSON (no change) |
| `confluence pull/push/create/update` | Print confirmation (no change) | Print confirmation (no change) |
| `jira list [--query] [--max] [--limit N]` | Show all or first N | Paged |
| `jira get <key>` | Print JSON (no change) | Print JSON (no change) |
| `jira create/transition` | Print confirmation (no change) | Print confirmation (no change) |
| `profile list/current` | Print (no change) | Print (no change) |
| `login` | Print confirmation (no change) | Print confirmation (no change) |

## `--limit` Flag

Add to commands that produce lists:
- `confluence pages` — `--limit <N>`: show first N pages (flat or tree)
- `confluence search` — `--limit <N>`: show first N results
- `jira list` — `--limit <N>`: show first N issues

Behavior:
- Without `--limit`: show all results (current behavior)
- With `--limit N`: show first N results
- In tree mode: `--limit` caps total nodes displayed (tree may appear incomplete if cut mid-branch)

## REPL Pager

### Display flow

```
(line 1)
(line 2)
...
(line 20)

--- Press Enter to continue, q to quit ---

(line 21)
(line 22)
...
(line 40)

--- Press Enter to continue, q to quit ---

(line 41)
(line 42)
(line 43)

atlas-cli(work)>
```

### Behavior

- Page size: 20 lines
- After each page, print prompt: `--- Press Enter to continue, q to quit ---`
- `Enter` (empty line) → show next page
- `q` or `Q` → stop, return to REPL prompt
- If total lines ≤ page size → no pager prompt, just display
- On last page → no prompt (back to REPL)

### Implementation

```rust
fn paged_print(lines: &[String], page_size: usize) {
    let total = lines.len();
    for (i, line) in lines.iter().enumerate() {
        println!("{}", line);
        if (i + 1) % page_size == 0 && i + 1 < total {
            print!("--- Press Enter to continue, q to quit ---");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            if input.trim().to_lowercase() == "q" {
                println!("(skipped {} remaining lines)", total - i - 1);
                break;
            }
        }
    }
}
```

## Files to Modify

| File | Changes |
|---|---|
| `src/output.rs` | Add `CommandOutput` enum, `print_all()`, `paged_print()` |
| `src/commands/confluence.rs` | Return `CommandOutput` from `execute()`. Add `--limit` to `Pages` and `Search` |
| `src/commands/jira.rs` | Return `CommandOutput` from `execute()`. Add `--limit` to `List` |
| `src/commands/profile.rs` | Return `CommandOutput` from `execute()` for consistency |
| `src/main.rs` | Call `output.print_all()` on CLI results |
| `src/repl.rs` | Call `paged_print()` on REPL results. Update help text |

## Implementation Order

1. Add `CommandOutput` enum + `print_all()` + `paged_print()` to `src/output.rs`
2. Refactor `src/commands/confluence.rs` — return `CommandOutput`, add `--limit`
3. Refactor `src/commands/jira.rs` — return `CommandOutput`, add `--limit`
4. Refactor `src/commands/profile.rs` — return `CommandOutput`
5. Update `src/main.rs` — call `.print_all()` on CLI results
6. Update `src/repl.rs` — call `paged_print()` on REPL results, update help text
7. Update `AGENTS.md`
8. Verify: `cargo fmt --check && cargo clippy && cargo test`

## Trade-offs

- **Refactor scope**: All `execute()` functions change return type. Small but touches every command.
- **Pager simplicity**: Uses `stdin.read_line()` for "Press Enter" — works in REPL, not ideal for piped input. Acceptable since REPL is interactive.
- **Tree view with --limit**: Caps total nodes displayed, not just root pages. Tree may appear incomplete if cut mid-branch. Acceptable for v1.
- **No page-up**: Pager only goes forward. Users can re-run command to see earlier results. Acceptable for v1.
