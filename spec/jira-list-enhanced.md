# Spec: Enhanced Jira List (Type, Assignee, URL, Subtasks)

**Status**: Implemented
**Created**: 2025-07-31

## Goal

Enhance `jira list` with more columns and subtask handling:

```bash
# Default: no subtasks, more columns
atlas-cli jira list

# Include subtasks in flat list
atlas-cli jira list --subtasks

# Grouped view with subtasks under parents
atlas-cli jira list --subtasks --tree
```

## Output Formats

### Default (no subtasks)

```
Key        Type    Summary                    Status       Assignee     URL
---------  ------  -------------------------  -----------  -----------  ----------------------------------------
PROJ-123   Story   Fix login bug              Done         John Doe     https://work.atlassian.net/browse/PROJ-123
PROJ-456   Bug     API timeout                Open         Jane Doe     https://work.atlassian.net/browse/PROJ-456
```

### With `--subtasks`

```
Key        Type      Summary              Status    Assignee    URL
---------  --------  -------------------  --------  ----------  ----------------------------------------
PROJ-123   Story     Fix login bug        Done      John Doe    https://work.atlassian.net/browse/PROJ-123
PROJ-124   Sub-task  Add validation       Done      John Doe    https://work.atlassian.net/browse/PROJ-124
PROJ-125   Sub-task  Write tests          In Prog   Jane Doe    https://work.atlassian.net/browse/PROJ-125
PROJ-456   Bug       API timeout          Open      Jane Doe    https://work.atlassian.net/browse/PROJ-456
```

### With `--subtasks --tree`

```
PROJ-123   Story     Fix login bug        Done      John Doe    https://work.atlassian.net/browse/PROJ-123
  ├─ PROJ-124  Sub-task  Add validation   Done      John Doe    https://work.atlassian.net/browse/PROJ-124
  └─ PROJ-125  Sub-task  Write tests      In Prog   Jane Doe    https://work.atlassian.net/browse/PROJ-125
PROJ-456   Bug       API timeout          Open      Jane Doe    https://work.atlassian.net/browse/PROJ-456
```

## Command Definition

```rust
/// List issues
List {
    /// JQL query
    #[arg(short, long)]
    query: Option<String>,

    /// Max results
    #[arg(short, long, default_value = "10")]
    max: u32,

    /// Limit displayed results
    #[arg(short, long)]
    limit: Option<usize>,

    /// Include subtasks
    #[arg(long)]
    subtasks: bool,

    /// Show subtasks grouped under parent
    #[arg(long)]
    tree: bool,
}
```

## Data Extraction from Jira API

```rust
// Type
i["fields"]["issuetype"]["name"].as_str().unwrap_or("")

// Is subtask?
i["fields"]["issuetype"]["subtask"].as_bool().unwrap_or(false)

// Assignee
i["fields"]["assignee"]["displayName"].as_str().unwrap_or("Unassigned")

// URL (constructed from client base_url)
format!("{}/browse/{}", client.base_url(), key)

// Parent key (for subtasks)
i["fields"]["parent"]["key"].as_str()
```

## Behavior

1. Fetch issues from Jira API with `expand=names` (to get field names)
2. Extract: key, type, summary, status, assignee, url, is_subtask, parent_key
3. **Default** (`--subtasks` not set): filter out subtasks (`is_subtask == true`)
4. **`--subtasks`**: include all issues
5. **`--subtasks --tree`**: group subtasks under parent issues using tree view
6. Apply `--limit` to final output

## Tree Building (for `--subtasks --tree`)

1. Collect all issues with metadata
2. Build `HashMap<parent_key, Vec<Issue>>` for subtasks
3. For each non-subtask issue, print it, then print its subtasks with `├─` / `└─` connectors
4. Subtask rows: `{connector} {key}  {type}  {summary}  {status}  {assignee}  {url}`

## Files to Modify

| File | Changes |
|---|---|
| `src/commands/jira.rs` | Add `--subtasks`, `--tree` flags to `List`. Add Type, Assignee, URL columns. Subtask filtering/grouping logic |

## Implementation Order

1. Add `--subtasks` and `--tree` flags to `JiraCommand::List`
2. Update `list()` to extract new fields (type, assignee, url, is_subtask, parent_key)
3. Add subtask filtering logic (default: hide, `--subtasks`: show)
4. Add tree grouping logic for `--subtasks --tree`
5. Update `execute()` match to pass new flags
6. Update REPL help text
7. Verify: `cargo fmt --check && cargo clippy && cargo test`
