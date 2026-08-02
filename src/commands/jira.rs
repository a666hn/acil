use std::collections::HashMap;

use clap::Parser;
use nu_ansi_term::{Color, Style};

use crate::client::ApiClient;
use crate::error::Result;
use crate::output::{self, CommandOutput};

fn key_style() -> Style {
    Style::new().fg(Color::Cyan).bold()
}

fn type_style(issue_type: &str) -> Style {
    match issue_type.to_lowercase().as_str() {
        "bug" => Style::new().fg(Color::Red),
        "story" => Style::new().fg(Color::Green),
        "task" => Style::new().fg(Color::Blue),
        "epic" => Style::new().fg(Color::Magenta),
        s if s.contains("sub") => Style::new().fg(Color::Purple),
        _ => Style::new(),
    }
}

fn status_style(status: &str) -> Style {
    let s = status.to_lowercase();
    if s.contains("done") || s.contains("closed") || s.contains("resolved") {
        Style::new().fg(Color::Green)
    } else if s.contains("progress") || s.contains("review") {
        Style::new().fg(Color::Yellow)
    } else if s.contains("block") || s.contains("fail") {
        Style::new().fg(Color::Red)
    } else {
        Style::new()
    }
}

fn assignee_style(assignee: &str) -> Style {
    if assignee == "Unassigned" {
        Style::new().fg(Color::DarkGray)
    } else {
        Style::new()
    }
}

fn url_style() -> Style {
    Style::new().fg(Color::DarkGray)
}

#[derive(Parser, Debug)]
pub enum JiraCommand {
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

        /// Filter by assignee email
        #[arg(short, long)]
        assigned: Option<String>,

        /// Filter by status (e.g. "To Do", "In Progress", "Done", "In Review", "On Hold")
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Get issue details
    Get {
        /// Issue key (e.g., PROJ-123)
        key: String,
    },

    /// Create a new issue
    Create {
        /// Project key
        #[arg(long)]
        project: String,

        /// Issue summary
        #[arg(short, long)]
        summary: String,

        /// Issue type (e.g., Bug, Story, Task, Sub-task)
        #[arg(short, long, default_value = "Task")]
        issue_type: String,

        /// Priority (e.g., Highest, High, Medium, Low, Lowest)
        #[arg(long)]
        priority: Option<String>,

        /// Comma-separated labels
        #[arg(short, long)]
        labels: Option<String>,

        /// Assignee (username or email)
        #[arg(short, long)]
        assignee: Option<String>,

        /// Parent issue key (for creating subtasks)
        #[arg(long)]
        parent: Option<String>,

        /// Description (markdown)
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Transition an issue
    Transition {
        /// Issue key
        key: String,

        /// Target status name
        #[arg(short, long)]
        status: String,
    },
}

struct IssueInfo {
    key: String,
    issue_type: String,
    summary: String,
    status: String,
    assignee: String,
    url: String,
    is_subtask: bool,
    parent_key: Option<String>,
}

pub async fn execute(client: &ApiClient, command: JiraCommand) -> Result<CommandOutput> {
    match command {
        JiraCommand::List {
            query,
            max,
            limit,
            subtasks,
            tree,
            assigned,
            status,
        } => list(client, query, max, limit, subtasks, tree, assigned, status).await,
        JiraCommand::Get { key } => get(client, &key).await,
        JiraCommand::Create {
            project,
            summary,
            issue_type,
            priority,
            labels,
            assignee,
            parent,
            description,
        } => {
            create(
                client,
                &project,
                &summary,
                &issue_type,
                priority.as_deref(),
                labels.as_deref(),
                assignee.as_deref(),
                parent.as_deref(),
                description.as_deref(),
            )
            .await
        }
        JiraCommand::Transition { key, status } => transition(client, &key, &status).await,
    }
}

async fn list(
    client: &ApiClient,
    query: Option<String>,
    max: u32,
    limit: Option<usize>,
    include_subtasks: bool,
    tree: bool,
    assigned: Option<String>,
    status: Option<String>,
) -> Result<CommandOutput> {
    let jql = query.unwrap_or_else(|| {
        let mut clauses = Vec::new();
        if let Some(a) = &assigned {
            clauses.push(format!("assignee = \"{}\"", a));
        }
        if let Some(s) = &status {
            clauses.push(format!("status = \"{}\"", s));
        }
        if clauses.is_empty() {
            // Jira's search API rejects a bare "ORDER BY" with no restriction at all
            // ("Unbounded JQL queries are not allowed here"), so this trivially-true
            // clause is here to satisfy that check without actually filtering anything.
            "issuetype is not EMPTY ORDER BY updated DESC".to_string()
        } else {
            format!("{} ORDER BY updated DESC", clauses.join(" AND "))
        }
    });
    let fields = "summary,status,assignee,issuetype,parent";

    // The search API caps how much a single page returns regardless of the
    // requested maxResults (observed: a request for 1000 still only returned
    // 179 with isLast=false), so satisfying `max` requires following
    // nextPageToken across as many requests as it takes.
    const PAGE_SIZE: u32 = 100;
    const SAFETY_PAGE_CAP: u32 = 20;

    let mut collected: Vec<serde_json::Value> = Vec::new();
    let mut next_token: Option<String> = None;
    let mut pages_fetched = 0;

    loop {
        let mut path = format!(
            "/rest/api/3/search/jql?jql={}&maxResults={}&fields={}",
            jql, PAGE_SIZE, fields
        );
        if let Some(tok) = &next_token {
            path.push_str(&format!("&nextPageToken={}", tok));
        }

        let resp = client
            .get(&path)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        // Print raw response in verbose mode
        if client.verbose() {
            eprintln!(
                "[verbose] Response: {}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            );
        }

        // Check for API errors
        if let Some(errors) = resp["errorMessages"].as_array()
            && !errors.is_empty()
        {
            let msgs: Vec<String> = errors
                .iter()
                .map(|e| e.as_str().unwrap_or("").to_string())
                .collect();
            return Err(crate::error::AppError::NotFound(format!(
                "Jira API error: {}",
                msgs.join(", ")
            )));
        }
        if let Some(msg) = resp["message"].as_str()
            && !msg.is_empty()
        {
            return Err(crate::error::AppError::NotFound(format!(
                "Jira API error: {}",
                msg
            )));
        }

        let page_issues = match resp["issues"].as_array() {
            Some(arr) if !arr.is_empty() => arr.clone(),
            _ => break,
        };

        for issue in page_issues {
            let is_subtask = issue["fields"]["issuetype"]["subtask"]
                .as_bool()
                .unwrap_or(false);
            if include_subtasks || !is_subtask {
                collected.push(issue);
            }
        }

        pages_fetched += 1;
        let hit_target = collected.len() >= max as usize;
        let is_last = resp["isLast"].as_bool().unwrap_or(true);
        next_token = resp["nextPageToken"].as_str().map(str::to_string);

        if hit_target || is_last || next_token.is_none() || pages_fetched >= SAFETY_PAGE_CAP {
            if pages_fetched >= SAFETY_PAGE_CAP && !hit_target {
                eprintln!(
                    "Note: stopped after scanning {} pages without reaching {} results — narrow with --status/--assigned/--query for a complete list.",
                    pages_fetched, max
                );
            }
            break;
        }
    }
    collected.truncate(max as usize);

    if collected.is_empty() {
        return Ok(output::collect_single_line(
            "No issues found. Try: acil jira list --query \"project = PROJ\"".to_string(),
        ));
    }

    let base_url = client.base_url();

    let all_issues: Vec<IssueInfo> = collected
        .iter()
        .map(|i| {
            let key = i["key"].as_str().unwrap_or("").to_string();
            IssueInfo {
                key: key.clone(),
                issue_type: i["fields"]["issuetype"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                summary: i["fields"]["summary"].as_str().unwrap_or("").to_string(),
                status: i["fields"]["status"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                assignee: i["fields"]["assignee"]["displayName"]
                    .as_str()
                    .unwrap_or("Unassigned")
                    .to_string(),
                url: format!("{}/browse/{}", base_url, key),
                is_subtask: i["fields"]["issuetype"]["subtask"]
                    .as_bool()
                    .unwrap_or(false),
                parent_key: i["fields"]["parent"]["key"].as_str().map(|s| s.to_string()),
            }
        })
        .collect();

    if tree && include_subtasks {
        Ok(build_jira_tree(&all_issues, limit))
    } else {
        let filtered: Vec<&IssueInfo> = if include_subtasks {
            all_issues.iter().collect()
        } else {
            all_issues.iter().filter(|i| !i.is_subtask).collect()
        };

        let mut rows: Vec<Vec<String>> = filtered
            .iter()
            .map(|i| {
                vec![
                    output::paint(&i.key, key_style()),
                    output::paint(&i.issue_type, type_style(&i.issue_type)),
                    i.summary.clone(),
                    output::paint(&i.status, status_style(&i.status)),
                    output::paint(&i.assignee, assignee_style(&i.assignee)),
                    output::paint(&i.url, url_style()),
                ]
            })
            .collect();

        if let Some(n) = limit {
            rows.truncate(n);
        }

        Ok(output::collect_table(
            &["Key", "Type", "Summary", "Status", "Assignee", "URL"],
            &rows,
        ))
    }
}

fn build_jira_tree(all_issues: &[IssueInfo], limit: Option<usize>) -> CommandOutput {
    let mut lines: Vec<String> = Vec::new();
    let mut subtask_map: HashMap<Option<String>, Vec<&IssueInfo>> = HashMap::new();

    for issue in all_issues {
        if issue.is_subtask {
            subtask_map
                .entry(issue.parent_key.clone())
                .or_default()
                .push(issue);
        }
    }

    let mut count = 0;
    let mut printed = 0;

    for issue in all_issues {
        if issue.is_subtask {
            continue;
        }

        if let Some(n) = limit
            && count >= n
        {
            break;
        }

        lines.push(format!(
            "{} {} {:<24} {} {} {}",
            output::paint(&format!("{:<10}", issue.key), key_style()),
            output::paint(
                &format!("{:<9}", issue.issue_type),
                type_style(&issue.issue_type)
            ),
            issue.summary,
            output::paint(
                &format!("{:<11}", issue.status),
                status_style(&issue.status)
            ),
            output::paint(
                &format!("{:<11}", issue.assignee),
                assignee_style(&issue.assignee)
            ),
            output::paint(&issue.url, url_style()),
        ));
        count += 1;

        if let Some(children) = subtask_map.get(&Some(issue.key.clone())) {
            for (i, child) in children.iter().enumerate() {
                if let Some(n) = limit
                    && count >= n
                {
                    break;
                }

                let connector = if i == children.len() - 1 {
                    "└─"
                } else {
                    "├─"
                };
                lines.push(format!(
                    "  {} {} {} {:<24} {} {} {}",
                    output::paint(connector, url_style()),
                    output::paint(&format!("{:<8}", child.key), key_style()),
                    output::paint(
                        &format!("{:<9}", child.issue_type),
                        type_style(&child.issue_type)
                    ),
                    child.summary,
                    output::paint(
                        &format!("{:<11}", child.status),
                        status_style(&child.status)
                    ),
                    output::paint(
                        &format!("{:<11}", child.assignee),
                        assignee_style(&child.assignee)
                    ),
                    output::paint(&child.url, url_style()),
                ));
                count += 1;
            }
        }

        printed += 1;
    }

    if printed == 0 {
        return CommandOutput::Empty;
    }

    CommandOutput::Lines(lines)
}

async fn get(client: &ApiClient, key: &str) -> Result<CommandOutput> {
    let path = format!("/rest/api/3/issue/{}", key);
    let resp = client
        .get(&path)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(output::collect_json(&resp))
}

async fn create(
    client: &ApiClient,
    project: &str,
    summary: &str,
    issue_type: &str,
    priority: Option<&str>,
    labels: Option<&str>,
    assignee: Option<&str>,
    parent: Option<&str>,
    description: Option<&str>,
) -> Result<CommandOutput> {
    let mut fields = serde_json::json!({
        "project": { "key": project },
        "summary": summary,
        "issuetype": { "name": issue_type }
    });

    if let Some(p) = priority {
        fields["priority"] = serde_json::json!({ "name": p });
    }

    if let Some(l) = labels {
        let label_vec: Vec<&str> = l
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        fields["labels"] = serde_json::json!(label_vec);
    }

    if let Some(a) = assignee {
        fields["assignee"] = serde_json::json!({ "name": a });
    }

    if let Some(p) = parent {
        fields["parent"] = serde_json::json!({ "key": p });
    }

    if let Some(md) = description {
        let adf = crate::jira_adf::markdown_to_adf(md);
        fields["description"] = adf;
    }

    let body = serde_json::json!({ "fields": fields });

    let resp = client
        .post("/rest/api/3/issue")
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(output::collect_single_line(format!(
        "Created: {}",
        resp["key"].as_str().unwrap_or("?")
    )))
}

async fn transition(client: &ApiClient, key: &str, status: &str) -> Result<CommandOutput> {
    let path = format!("/rest/api/3/issue/{}/transitions", key);
    let transitions: serde_json::Value = client.get(&path).send().await?.json().await?;

    let transition_id = transitions["transitions"]
        .as_array()
        .and_then(|ts| {
            ts.iter()
                .find(|t| t["name"].as_str() == Some(status))
                .and_then(|t| t["id"].as_str())
        })
        .ok_or_else(|| {
            crate::error::AppError::NotFound(format!("Transition '{}' not found", status))
        })?;

    let body = serde_json::json!({
        "transition": { "id": transition_id }
    });
    client.post(&path).json(&body).send().await?;
    Ok(output::collect_single_line(format!(
        "Transitioned {} to '{}'",
        key, status
    )))
}
