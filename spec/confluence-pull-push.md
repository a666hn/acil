# Spec: Confluence Pull/Push Local Edit Workflow

**Status**: Implemented
**Created**: 2025-07-31

## Goal

Enable editing Confluence pages locally as markdown files:

```bash
atlas-cli confluence pull 12345678           # page → local .md file
vim 12345678.md                              # edit with any editor
atlas-cli confluence push 12345678.md        # .md file → update Confluence page
```

Plus template support:

```bash
vim my-template.md
atlas-cli confluence create --space PROJ --title "New Page" --file my-template.md
```

## Local File Format

Each pulled page is a self-contained `.md` file with YAML frontmatter:

```markdown
---
page_id: "12345678"
title: "My Page Title"
space: "PROJ"
version: 5
url: "https://domain.atlassian.net/wiki/pages/viewpage.action?pageId=12345678"
---

# My Page Title

Page content in **markdown**...
```

- Default filename: `{page-id}.md` in current directory
- `--output <dir>` to choose output directory

## New Dependencies

```toml
pulldown-cmark = "0.13"   # markdown parser → event-based, custom XHTML renderer
htmd = "0.5"              # HTML/XHTML → markdown, supports custom element handlers
```

## Commands

### `confluence pull <page-id> [--output <dir>]`

1. Fetch page: `GET /wiki/rest/api/content/{id}?expand=body.storage,version,space,ancestors`
2. Convert storage format → markdown via `storage_to_markdown()`
3. Build `PageMeta` from API response (page_id, title, space, version, url)
4. Write `{page_id}.md` with frontmatter + markdown body
5. Print: `Pulled "Page Title" → 12345678.md (v5)`

### `confluence push <file>`

1. Read `.md` file, parse frontmatter → `PageMeta` + markdown body
2. Convert markdown → storage format via `markdown_to_storage()`
3. Fetch current page from API to get server version
4. Conflict check:
   - If `server_version > local_version` → error: `"Page was updated on Confluence (local: v5, server: v7). Run 'confluence pull' first."`
   - If `server_version == local_version` → proceed
5. PUT update with `version = local_version + 1`
6. Print: `Pushed 12345678.md → "Page Title" (v6)`

### `confluence create --space <key> --title <title> [--file <path>] [--parent <id>]`

- With `--file`: read markdown file, convert to storage format, use as page body
- Without `--file`: create empty page (existing behavior)
- Title comes from `--title` flag, not from file

## Conversion Details

### Markdown → Confluence Storage (`markdown_to_storage`)

Uses `pulldown-cmark::Parser` (event iterator) → custom XHTML renderer.

| Markdown | Confluence Storage |
|---|---|
| `# heading` | `<h1>heading</h1>` |
| `**bold**` | `<strong>bold</strong>` |
| `*italic*` | `<em>italic</em>` |
| `[text](url)` | `<a href="url">text</a>` |
| `` `code` `` | `<code>code</code>` |
| Code fences | `<ac:structured-macro ac:name="code"><ac:plain-text-body><![CDATA[...]]></ac:plain-text-body></ac:structured-macro>` |
| `- item` | `<ul><li>item</li></ul>` |
| `1. item` | `<ol><li>item</li></ol>` |
| Tables | `<table><tr><td>...</td></tr></table>` |
| `![alt](url)` | `<ac:image><ri:url ri:value="url"/></ac:image>` |
| `> quote` | `<blockquote>quote</blockquote>` |

### Confluence Storage → Markdown (`storage_to_markdown`)

Uses `htmd::HtmlToMarkdown` with custom handlers.

| Confluence Element | Markdown Output |
|---|---|
| `<h1>`-`<h6>` | `#` - `######` |
| `<strong>` | `**text**` |
| `<em>` | `*text*` |
| `<a href="url">` | `[text](url)` |
| `<code>` | `` `code` `` |
| `<ac:structured-macro ac:name="code">` | Fenced code block (```` ``` ````) |
| `<ac:structured-macro ac:name="info/warning/note">` | `> **Info:** text` blockquote |
| `<ac:image>` | `![alt](url)` |
| `<ul>`, `<ol>`, `<li>` | `- item`, `1. item` |
| `<table>` | Markdown table |
| `<blockquote>` | `> quote` |
| `<ri:attachment>` | `[filename](attachment:filename)` |

## New Files

| File | Purpose |
|---|---|
| `src/confluence_meta.rs` | `PageMeta` struct, `parse()` / `to_frontmatter()` |
| `src/confluence_convert.rs` | `markdown_to_storage()`, `storage_to_markdown()` |

## Modified Files

| File | Changes |
|---|---|
| `Cargo.toml` | Add `pulldown-cmark`, `htmd` |
| `src/commands/confluence.rs` | Add `Pull`, `Push` variants; update `Create` with `--file`; update `execute` match |
| `src/repl.rs` | Update help text with new commands |

No changes needed: `main.rs`, `client.rs`, `config.rs`, `error.rs`, `output.rs`.

## Implementation Order

1. Add deps to `Cargo.toml`
2. Create `src/confluence_meta.rs`
3. Create `src/confluence_convert.rs`
4. Update `src/commands/confluence.rs`
5. Update `src/repl.rs` help text
6. Verify: `cargo fmt --check && cargo clippy && cargo test`

## Known Limitations (v1)

- **No attachments** — images by URL work, file attachments not downloaded/uploaded
- **Confluence macros** — complex macros (Jira panels, page includes, draw.io) preserved as-is but no clean markdown equivalent
- **Roundtrip fidelity** — basic content roundtrips cleanly; very complex nested HTML may have minor formatting differences
