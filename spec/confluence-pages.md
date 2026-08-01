# Spec: Confluence Pages (Hierarchical Page Listing)

**Status**: Implemented
**Created**: 2025-07-31

## Goal

List pages in a Confluence space with optional tree view for nested pages:

```bash
atlas-cli confluence pages --space PROJ              # flat table
atlas-cli confluence pages --space PROJ --tree        # indented tree view
```

## Display Format

### Flat (default)

```
ID        Title                    Type
---       -----                    ----
123456    Getting Started          page
234567    API Reference            page
345678    Changelog                page
```

Reuses existing `output::print_table()`.

### Tree (`--tree`)

```
Pages in PROJ — 12 pages, 3 levels deep
───────────────────────────────────────
Getting Started (123456)
├─ Installation (234567)
└─ Configuration (345678)
   ├─ Environment Variables (456789)
   └─ Auth Setup (567890)
API Reference (111222)
├─ REST API (333444)
└─ GraphQL (555666)
```

- Title is primary, ID in parentheses is secondary
- Box-drawing characters: `├─`, `└─`, continuation `│` or spaces
- Header: `Pages in {SPACE} — {count} pages, {depth} levels deep`
- Separator: `─────────────` (matches header width)
- Root pages at column 0, each nesting level adds 3 chars indent (`│  ` or `   `)

## Command

```rust
/// List pages in a space
Pages {
    /// Space key (e.g., PROJ)
    #[arg(short, long)]
    space: String,

    /// Show as indented tree
    #[arg(short, long)]
    tree: bool,

    /// Max results per page (default: 250)
    #[arg(short, long, default_value = "250")]
    max: u32,
}
```

## API

v1 API with CQL, paginate via `start`:

```
GET /wiki/rest/api/content/search
  ?cql=space="{spaceKey}"+and+type=page
  &expand=ancestors
  &limit={max}
  &start=0
```

Each page in response includes `ancestors` array:
- Empty `[]` → root page
- Last element → direct parent ID

Paginate: increment `start` by `limit` until `results` is empty or `size` < `limit`.

## Data Structures

```rust
struct FlatPage {
    id: String,
    title: String,
    parent_id: Option<String>,
}

struct TreeNode {
    title: String,
    id: String,
    children: Vec<TreeNode>,
}
```

## Tree Building Algorithm

1. Fetch all pages via paginated CQL with `expand=ancestors`
2. For each page, build `FlatPage`:
   - `id` = `page["id"]`
   - `title` = `page["title"]`
   - `parent_id` = last element of `page["ancestors"]` array, or `None` if empty
3. Build adjacency list: `HashMap<Option<String>, Vec<FlatPage>>`
4. Collect roots: entries where `parent_id = None`
5. For each root, recursively build `TreeNode` from adjacency list

## Tree Rendering (`print_tree`)

Function signature:
```rust
pub fn print_tree(header: &str, roots: &[TreeNode])
```

Rendering:
1. Print header line
2. Print separator line (`─` repeated to match header width)
3. DFS traversal of tree nodes
4. Track prefix state at each depth level:
   - `is_last = true` → prefix `└─ `, continuation `   `
   - `is_last = false` → prefix `├─ `, continuation `│  `
5. Each node prints: `{prefix}{title} ({id})`

## Files to Modify

| File | Changes |
|---|---|
| `src/commands/confluence.rs` | Add `Pages` variant, `FlatPage` struct, `pages()` function, `build_tree()` |
| `src/output.rs` | Add `TreeNode` struct, `print_tree()` function |
| `src/repl.rs` | Add `confluence pages` to help text |
| `AGENTS.md` | Add command reference |

## Implementation Order

1. Add `TreeNode` + `print_tree()` to `src/output.rs`
2. Add `Pages` variant + `pages()` + `build_tree()` to `src/commands/confluence.rs`
3. Update `src/repl.rs` help text
4. Update `AGENTS.md`
5. Verify: `cargo fmt --check && cargo clippy && cargo test`
