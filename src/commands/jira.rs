use std::collections::HashMap;

use clap::Parser;

use crate::client::ApiClient;
use crate::error::Result;
use crate::output::{self, CommandOutput};

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
    },

    /// Get issue details
    Get {
        /// Issue key (e.g., PROJ-123)
        key: String,
    },

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
        } => list(client, query, max, limit, subtasks, tree).await,
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
) -> Result<CommandOutput> {
    let jql = query.unwrap_or_else(|| "assignee = currentUser() ORDER BY updated DESC".into());
    let path = format!("/rest/api/3/search?jql={}&maxResults={}", jql, max);
    let resp = client
        .get(&path)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let issues = match resp["issues"].as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => return Ok(CommandOutput::Empty),
    };

    let base_url = client.base_url();

    let all_issues: Vec<IssueInfo> = issues
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
                    i.key.clone(),
                    i.issue_type.clone(),
                    i.summary.clone(),
                    i.status.clone(),
                    i.assignee.clone(),
                    i.url.clone(),
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
            "{:<10} {:<9} {:<24} {:<11} {:<11} {}",
            issue.key, issue.issue_type, issue.summary, issue.status, issue.assignee, issue.url
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
                    "  {} {:<8} {:<9} {:<24} {:<11} {:<11} {}",
                    connector,
                    child.key,
                    child.issue_type,
                    child.summary,
                    child.status,
                    child.assignee,
                    child.url
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
