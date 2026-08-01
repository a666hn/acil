# Spec: REPL-Only Login and Profile Switch

**Status**: Implemented
**Created**: 2025-07-31

## Goal

Restrict `login` and `profile switch` to REPL mode only. These are interactive/auth operations that belong in the REPL, not CLI one-shot mode.

## Command Availability

| Command | CLI | REPL |
|---|---|---|
| `login --name --url --email` | ❌ | ✅ |
| `profile switch <name>` | ❌ | ✅ |
| `profile list` | ✅ | ✅ |
| `profile current` | ✅ | ✅ |
| `profile rm <name>` | ✅ | ✅ |

## Rationale

- `login` prompts for API token via `rpassword` — interactive, not suitable for CLI piping
- `profile switch` is a persistent state change — safer in interactive context
- `profile list/current/rm` are read-only or explicit removal — safe for CLI

## Files to Modify

| File | Changes |
|---|---|
| `src/main.rs` | Remove `Login` variant from `Commands` enum and match arm |
| `src/commands/profile.rs` | Remove `Switch` variant from `ProfileCommand`, remove from `execute()` |
| `src/repl.rs` | Handle `profile switch` manually in dispatch before parsing `ProfileCommand` |

## Implementation

### `src/main.rs`

- Remove `Login` variant from `Commands` enum
- Remove `Some(Commands::Login { .. }) => login(...)` match arm
- Keep `login()` async function (still called from REPL)
- Keep `rpassword` dependency

### `src/commands/profile.rs`

- Remove `Switch { name }` variant from `ProfileCommand` enum
- Remove `ProfileCommand::Switch { name } => switch(config, &name)` from `execute()`
- Keep `switch()` as a public function (called from REPL directly)

### `src/repl.rs`

- `login` already handled manually in dispatch (no change)
- For `profile switch`: intercept before `ProfileCommand::try_parse_from`, handle manually

```rust
"profile" => {
    if parts.len() >= 2 && parts[1] == "switch" {
        if parts.len() >= 3 {
            config.switch_profile(parts[2])?;
            config.save()?;
            output::collect_single_line(format!("Switched to profile '{}'", parts[2]))
                .paged_print(PAGE_SIZE);
        } else {
            eprintln!("Usage: profile switch <name>");
        }
        Ok(())
    } else {
        let args = std::iter::once("profile").chain(parts[1..].iter().copied());
        match profile::ProfileCommand::try_parse_from(args) {
            Ok(cmd) => {
                let out = profile::execute(config, cmd)?;
                out.paged_print(PAGE_SIZE);
                Ok(())
            }
            Err(e) => {
                eprintln!("{}", e);
                Ok(())
            }
        }
    }
}
```

## Verification Order

1. `cargo fmt --check`
2. `cargo clippy`
3. `cargo test`
