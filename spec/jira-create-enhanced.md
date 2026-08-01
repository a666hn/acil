# Spec: Enhanced Jira Create (Priority, Labels, Assignee, Parent, Description)

**Status**: Implemented
**Created**: 2025-07-31

## Goal

Enhance `jira create` with more fields and markdown-to-ADF conversion for description:

```bash
atlas-cli jira create \
  --project PROJ \
  --summary "Fix login bug" \
  --issue-type Story \
  --priority High \
  --labels "bug,critical" \
  --assignee "john.doe@company.com" \
  --parent PROJ-100 \
  --description "## Steps\n\n1. Go to login\n2. Click submit\n\n**Expected:** Login works"
```

## New Command Definition

```rust
/// Create a new issue
Create {
    /// Project key
    #[arg(short, long)]
    project: String,

    /// Issue summary
    #[arg(short, long)]
    summary: String,

    /// Issue type (e.g., Bug, Story, Task, Sub-task)
    #[arg(short, long, default_value = "Task")]
    issue_type: String,

    /// Priority (e.g., Highest, High, Medium, Low, Lowest)
    #[arg(short, long)]
    priority: Option<String>,

    /// Comma-separated labels
    #[arg(short, long)]
    labels: Option<String>,

    /// Assignee (username or email)
    #[arg(short, long)]
    assignee: Option<String>,

    /// Parent issue key (for creating subtasks)
    #[arg(short, long)]
    parent: Option<String>,

    /// Description (markdown)
    #[arg(short, long)]
    description: Option<String>,
}
```

## API Request Body

```json
{
  "fields": {
    "project": { "key": "PROJ" },
    "summary": "Fix login bug",
    "issuetype": { "name": "Story" },
    "priority": { "name": "High" },
    "labels": ["bug", "critical"],
    "assignee": { "name": "john.doe@company.com" },
    "parent": { "key": "PROJ-100" },
    "description": {
      "version": 1,
      "type": "doc",
      "content": [
        {
          "type": "heading",
          "attrs": { "level": 2 },
          "content": [{ "type": "text", "text": "Steps" }]
        },
        {
          "type": "orderedList",
          "content": [
            {
              "type": "listItem",
              "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "Go to login" }] }
              ]
            },
            {
              "type": "listItem",
              "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "Click submit" }] }
              ]
            }
          ]
        },
        {
          "type": "paragraph",
          "content": [
            { "type": "text", "text": "Expected: " },
            { "type": "text", "text": "Login works", "marks": [{ "type": "strong" }] }
          ]
        }
      ]
    }
  }
}
```

## Description Conversion: Markdown → ADF

Jira Cloud API v3 uses **Atlassian Document Format (ADF)** for description — a JSON-based structured format, NOT plain text or HTML.

We need a new function: `markdown_to_adf(md: &str) -> serde_json::Value`

Uses `pulldown-cmark::Parser` (already a dependency) to parse markdown and build ADF JSON.

### ADF Node Types

| Markdown | ADF Node Type | Structure |
|---|---|---|
| `# heading` | `heading` | `{ "type": "heading", "attrs": { "level": N }, "content": [...] }` |
| Paragraph | `paragraph` | `{ "type": "paragraph", "content": [...] }` |
| `**bold**` | text + mark | `{ "type": "text", "text": "...", "marks": [{ "type": "strong" }] }` |
| `*italic*` | text + mark | `{ "type": "text", "text": "...", "marks": [{ "type": "em" }] }` |
| `` `code` `` | text + mark | `{ "type": "text", "text": "...", "marks": [{ "type": "code" }] }` |
| `[text](url)` | text + mark | `{ "type": "text", "text": "...", "marks": [{ "type": "link", "attrs": { "href": "..." } }] }` |
| `- item` | `bulletList` | `{ "type": "bulletList", "content": [{ "type": "listItem", "content": [...] }] }` |
| `1. item` | `orderedList` | `{ "type": "orderedList", "content": [{ "type": "listItem", "content": [...] }] }` |
| Code fence | `codeBlock` | `{ "type": "codeBlock", "attrs": { "language": "..." }, "content": [{ "type": "text" }] }` |
| `> quote` | `blockquote` | `{ "type": "blockquote", "content": [...] }` |
| `---` | `rule` | `{ "type": "rule" }` |

### Root ADF Document

```json
{
  "version": 1,
  "type": "doc",
  "content": [
    // ... array of block nodes
  ]
}
```

### Converter Approach

Use `pulldown-cmark::Parser` (same as `confluence_convert.rs`):
1. Iterate markdown events
2. Build ADF node tree using `serde_json::Value`
3. Track state: current block node, inline content buffer, list stack
4. On block end (paragraph, heading, list item): push completed node to content array
5. On inline content (text, code, links): push to current block's content array with marks

## Files to Modify/Create

| File | Changes |
|---|---|
| `src/jira_adf.rs` | **New**: `markdown_to_adf()` function |
| `src/main.rs` | Register `mod jira_adf` |
| `src/commands/jira.rs` | Add new flags to `Create`. Update `create()` to build ADF and include new fields |
| `src/repl.rs` | Update help text |

## Implementation Order

1. Create `src/jira_adf.rs` with `markdown_to_adf()`
2. Register `mod jira_adf` in `src/main.rs`
3. Add new flags to `JiraCommand::Create` in `src/commands/jira.rs`
4. Update `create()` to build request body with ADF and new fields
5. Update REPL help text
6. Verify: `cargo fmt --check && cargo clippy && cargo test`

## Field Handling Details

### `--priority`
- Pass as `priority.name` in API body
- Common values: Highest, High, Medium, Low, Lowest
- API accepts name string, no need for ID

### `--labels`
- Comma-separated string, split into `Vec<String>`
- Pass as `labels: ["l1", "l2"]` in API body

### `--assignee`
- Pass as `assignee.name` in API body
- Accepts username or email (Jira Cloud resolves it)
- Note: Jira Cloud prefers `accountId`, but `name` still works for email-based auth

### `--parent`
- Pass as `parent.key` in API body
- Required for Sub-task issue type
- Example: `--parent PROJ-100`

### `--description`
- Markdown string, convert to ADF via `markdown_to_adf()`
- If not provided, omit `description` field (API default)

## Output on Success

```
Created: PROJ-123
```

(Same as current behavior)
