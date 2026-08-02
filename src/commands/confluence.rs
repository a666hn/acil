use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;
use nu_ansi_term::{Color, Style};

use crate::client::ApiClient;
use crate::confluence_convert;
use crate::confluence_meta::{self, PageMeta};
use crate::error::{AppError, Result};
use crate::output::{self, CommandOutput, TreeNode};

fn id_style() -> Style {
    Style::new().fg(Color::DarkGray)
}

fn page_type_style(page_type: &str) -> Style {
    match page_type.to_lowercase().as_str() {
        "page" => Style::new().fg(Color::Blue),
        "blogpost" => Style::new().fg(Color::Magenta),
        _ => Style::new(),
    }
}

#[derive(Parser, Debug)]
pub enum ConfluenceCommand {
    /// Search pages
    Search {
        /// Search query (CQL)
        query: String,

        /// Max results
        #[arg(short, long, default_value = "100")]
        max: u32,

        /// Limit displayed results
        #[arg(short, long)]
        limit: Option<usize>,
    },

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

        /// Limit displayed results
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Get page content
    Get {
        /// Page ID
        id: String,
    },

    /// Download a page to a local markdown file
    Pull {
        /// Page ID
        id: String,

        /// Output directory (default: current directory)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Push a local markdown file to Confluence
    Push {
        /// Path to the markdown file
        file: String,
    },

    /// Create a new page
    Create {
        /// Space key
        #[arg(short, long)]
        space: String,

        /// Page title
        #[arg(short, long)]
        title: String,

        /// Create from a local markdown file
        #[arg(short, long)]
        file: Option<String>,

        /// Parent page ID (optional)
        #[arg(long)]
        parent: Option<String>,
    },

    /// Update page content
    Update {
        /// Page ID
        id: String,

        /// New title (optional)
        #[arg(short, long)]
        title: Option<String>,

        /// New content (body)
        #[arg(short, long)]
        body: Option<String>,
    },
}

pub async fn execute(client: &ApiClient, command: ConfluenceCommand) -> Result<CommandOutput> {
    match command {
        ConfluenceCommand::Search { query, max, limit } => search(client, &query, max, limit).await,
        ConfluenceCommand::Pages {
            space,
            tree,
            max,
            limit,
        } => pages(client, &space, tree, max, limit).await,
        ConfluenceCommand::Get { id } => get(client, &id).await,
        ConfluenceCommand::Pull { id, output } => pull(client, &id, output.as_deref()).await,
        ConfluenceCommand::Push { file } => push(client, &file).await,
        ConfluenceCommand::Create {
            space,
            title,
            file,
            parent,
        } => create(client, &space, &title, file.as_deref(), parent.as_deref()).await,
        ConfluenceCommand::Update { id, title, body } => {
            update(client, &id, title.as_deref(), body.as_deref()).await
        }
    }
}

async fn search(
    client: &ApiClient,
    query: &str,
    max: u32,
    limit: Option<usize>,
) -> Result<CommandOutput> {
    let path = format!(
        "/wiki/rest/api/content/search?cql=text~\"{}\"&limit={}",
        query, max
    );
    let resp = client
        .get(&path)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    // Check for API errors
    if let Some(msg) = resp["message"].as_str()
        && !msg.is_empty()
    {
        return Err(crate::error::AppError::NotFound(format!(
            "Confluence API error: {}",
            msg
        )));
    }
    if let Some(errors) = resp["errorMessages"].as_array()
        && !errors.is_empty()
    {
        let msgs: Vec<String> = errors
            .iter()
            .map(|e| e.as_str().unwrap_or("").to_string())
            .collect();
        return Err(crate::error::AppError::NotFound(format!(
            "Confluence API error: {}",
            msgs.join(", ")
        )));
    }

    if let Some(results) = resp["results"].as_array() {
        let mut rows: Vec<Vec<String>> = results
            .iter()
            .map(|r| {
                let page_type = r["type"].as_str().unwrap_or("");
                vec![
                    output::paint(r["id"].as_str().unwrap_or(""), id_style()),
                    r["title"].as_str().unwrap_or("").to_string(),
                    output::paint(page_type, page_type_style(page_type)),
                ]
            })
            .collect();

        if let Some(n) = limit {
            rows.truncate(n);
        }

        if rows.is_empty() {
            return Ok(output::collect_single_line(format!(
                "No pages found matching '{}'",
                query
            )));
        }

        Ok(output::collect_table(&["ID", "Title", "Type"], &rows))
    } else {
        Ok(output::collect_single_line(format!(
            "No pages found matching '{}'",
            query
        )))
    }
}

struct FlatPage {
    id: String,
    title: String,
    parent_id: Option<String>,
}

async fn pages(
    client: &ApiClient,
    space: &str,
    tree: bool,
    max: u32,
    limit: Option<usize>,
) -> Result<CommandOutput> {
    let mut all_pages: Vec<FlatPage> = Vec::new();
    let mut start: u32 = 0;

    loop {
        let path = format!(
            "/wiki/rest/api/content/search?cql=space=\"{}\"+and+type=page&expand=ancestors&limit={}&start={}",
            space, max, start
        );
        let resp: serde_json::Value = client.get(&path).send().await?.json().await?;

        // Check for API errors
        if let Some(msg) = resp["message"].as_str()
            && !msg.is_empty()
        {
            return Err(crate::error::AppError::NotFound(format!(
                "Confluence API error: {}",
                msg
            )));
        }
        if let Some(errors) = resp["errorMessages"].as_array()
            && !errors.is_empty()
        {
            let msgs: Vec<String> = errors
                .iter()
                .map(|e| e.as_str().unwrap_or("").to_string())
                .collect();
            return Err(crate::error::AppError::NotFound(format!(
                "Confluence API error: {}",
                msgs.join(", ")
            )));
        }

        let results = match resp["results"].as_array() {
            Some(r) if !r.is_empty() => r.clone(),
            _ => break,
        };

        let count = results.len() as u32;

        for page in &results {
            let id = page["id"].as_str().unwrap_or("").to_string();
            let title = page["title"].as_str().unwrap_or("").to_string();
            let parent_id = page["ancestors"]
                .as_array()
                .and_then(|a| a.last())
                .and_then(|p| p["id"].as_str())
                .map(|s| s.to_string());

            all_pages.push(FlatPage {
                id,
                title,
                parent_id,
            });
        }

        start += count;
        if count < max {
            break;
        }
    }

    if all_pages.is_empty() {
        return Ok(output::collect_single_line(format!(
            "No pages found in space '{}'",
            space
        )));
    }

    if tree {
        let (roots, max_depth) = build_tree(&all_pages, limit);
        let total = limit.unwrap_or(all_pages.len()).min(all_pages.len());
        let header = format!(
            "Pages in {} — {} pages, {} levels deep",
            space, total, max_depth
        );
        Ok(output::collect_tree(&header, &roots))
    } else {
        let mut rows: Vec<Vec<String>> = all_pages
            .iter()
            .map(|p| vec![output::paint(&p.id, id_style()), p.title.clone()])
            .collect();

        if let Some(n) = limit {
            rows.truncate(n);
        }

        Ok(output::collect_table(&["ID", "Title"], &rows))
    }
}

fn build_tree(pages: &[FlatPage], limit: Option<usize>) -> (Vec<TreeNode>, usize) {
    let mut children_map: HashMap<Option<String>, Vec<&FlatPage>> = HashMap::new();
    for page in pages {
        children_map
            .entry(page.parent_id.clone())
            .or_default()
            .push(page);
    }

    fn build_subtree(
        parent_id: &Option<String>,
        children_map: &HashMap<Option<String>, Vec<&FlatPage>>,
        depth: usize,
        remaining: &mut Option<usize>,
    ) -> (Vec<TreeNode>, usize) {
        let mut nodes = Vec::new();
        let mut max_depth = depth;

        if let Some(children) = children_map.get(parent_id) {
            for child in children {
                if let Some(r) = remaining {
                    if *r == 0 {
                        break;
                    }
                    *r -= 1;
                }

                let child_id = Some(child.id.clone());
                let (sub_children, sub_depth) =
                    build_subtree(&child_id, children_map, depth + 1, remaining);
                max_depth = max_depth.max(sub_depth);
                nodes.push(TreeNode {
                    title: child.title.clone(),
                    id: child.id.clone(),
                    children: sub_children,
                });
            }
        }

        (nodes, max_depth)
    }

    let mut remaining = limit;
    build_subtree(&None, &children_map, 1, &mut remaining)
}

async fn get(client: &ApiClient, id: &str) -> Result<CommandOutput> {
    let path = format!("/wiki/rest/api/content/{}?expand=body.storage", id);
    let resp = client
        .get(&path)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    // Check for API errors
    if let Some(msg) = resp["message"].as_str()
        && !msg.is_empty()
    {
        return Err(crate::error::AppError::NotFound(format!(
            "Confluence API error: {}",
            msg
        )));
    }

    Ok(output::collect_json(&resp))
}

async fn pull(client: &ApiClient, id: &str, output_dir: Option<&str>) -> Result<CommandOutput> {
    let path = format!(
        "/wiki/rest/api/content/{}?expand=body.storage,version,space,ancestors",
        id
    );
    let resp: serde_json::Value = client.get(&path).send().await?.json().await?;

    let title = resp["title"].as_str().unwrap_or("Untitled");
    let storage = resp["body"]["storage"]["value"].as_str().unwrap_or("");
    let version = resp["version"]["number"].as_i64().unwrap_or(1);
    let space_key = resp["space"]["key"].as_str().unwrap_or("");
    let base_url = client.base_url();
    let url = PageMeta::build_url(base_url, id);

    let meta = PageMeta {
        page_id: id.to_string(),
        title: title.to_string(),
        space: space_key.to_string(),
        version,
        url,
    };

    let markdown = confluence_convert::storage_to_markdown(storage);
    let file_content = format!("{}\n{}", meta.to_frontmatter(), markdown);

    let file_path = if let Some(dir) = output_dir {
        PathBuf::from(dir).join(format!("{}.md", id))
    } else {
        PathBuf::from(format!("{}.md", id))
    };

    std::fs::write(&file_path, &file_content).map_err(AppError::Io)?;

    Ok(output::collect_single_line(format!(
        "Pulled \"{}\" → {} (v{})",
        title,
        file_path.display(),
        version
    )))
}

async fn push(client: &ApiClient, file_path: &str) -> Result<CommandOutput> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| AppError::Config(format!("Failed to read {}: {}", file_path, e)))?;

    let (meta, body) = confluence_meta::parse(&content)?;
    let storage_value = confluence_convert::markdown_to_storage(body);

    let path = format!(
        "/wiki/rest/api/content/{}?expand=version,space",
        meta.page_id
    );
    let current: serde_json::Value = client.get(&path).send().await?.json().await?;

    let server_version = current["version"]["number"].as_i64().unwrap_or(1);
    if server_version > meta.version {
        return Err(AppError::Config(format!(
            "Page was updated on Confluence (local: v{}, server: v{}). Run 'confluence pull' first.",
            meta.version, server_version
        )));
    }

    let new_version = server_version + 1;
    let update_body = serde_json::json!({
        "id": meta.page_id,
        "type": "page",
        "title": meta.title,
        "version": { "number": new_version },
        "body": {
            "storage": {
                "value": storage_value,
                "representation": "storage"
            }
        }
    });

    let update_path = format!("/wiki/rest/api/content/{}", meta.page_id);
    client.put(&update_path).json(&update_body).send().await?;

    Ok(output::collect_single_line(format!(
        "Pushed {} → \"{}\" (v{})",
        file_path, meta.title, new_version
    )))
}

async fn create(
    client: &ApiClient,
    space: &str,
    title: &str,
    file: Option<&str>,
    parent: Option<&str>,
) -> Result<CommandOutput> {
    let body_value = if let Some(file_path) = file {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| AppError::Config(format!("Failed to read {}: {}", file_path, e)))?;
        confluence_convert::markdown_to_storage(&content)
    } else {
        String::new()
    };

    let mut body = serde_json::json!({
        "type": "page",
        "title": title,
        "space": { "key": space },
        "body": {
            "storage": {
                "value": body_value,
                "representation": "storage"
            }
        }
    });

    if let Some(parent_id) = parent {
        body["ancestors"] = serde_json::json!([{ "id": parent_id }]);
    }

    let resp = client
        .post("/wiki/rest/api/content")
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(output::collect_single_line(format!(
        "Created page: {} ({})",
        title,
        resp["id"].as_str().unwrap_or("?")
    )))
}

async fn update(
    client: &ApiClient,
    id: &str,
    title: Option<&str>,
    body: Option<&str>,
) -> Result<CommandOutput> {
    let path = format!("/wiki/rest/api/content/{}", id);
    let current: serde_json::Value = client.get(&path).send().await?.json().await?;

    let version = current["version"]["number"].as_i64().unwrap_or(1) + 1;

    let mut update_body = serde_json::json!({
        "id": id,
        "type": current["type"],
        "title": title.unwrap_or(current["title"].as_str().unwrap_or("")),
        "version": { "number": version },
        "body": {
            "storage": {
                "value": body.unwrap_or(""),
                "representation": "storage"
            }
        }
    });

    if title.is_none() {
        update_body["title"] = current["title"].clone();
    }
    if body.is_none() {
        update_body["body"] = current["body"].clone();
    }

    let space_key = current["space"]["key"].as_str().unwrap_or("");
    update_body["space"] = serde_json::json!({ "key": space_key });

    client.put(&path).json(&update_body).send().await?;
    Ok(output::collect_single_line(format!("Updated page {}", id)))
}
