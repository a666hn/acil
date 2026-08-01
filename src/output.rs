use std::io::{self, Write};

use serde_json::Value;

#[derive(Debug)]
pub enum CommandOutput {
    Lines(Vec<String>),
    Empty,
}

impl CommandOutput {
    pub fn print_all(&self) {
        if let CommandOutput::Lines(lines) = self {
            for line in lines {
                println!("{}", line);
            }
        }
    }

    pub fn paged_print(&self, page_size: usize) {
        if let CommandOutput::Lines(lines) = self {
            let total = lines.len();
            if total == 0 {
                return;
            }
            for (i, line) in lines.iter().enumerate() {
                println!("{}", line);
                if (i + 1) % page_size == 0 && i + 1 < total {
                    print!("--- Press Enter to continue, q to quit ---");
                    io::stdout().flush().ok();
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).ok();
                    if input.trim().to_lowercase() == "q" {
                        println!("(skipped {} remaining lines)", total - i - 1);
                        break;
                    }
                }
            }
        }
    }
}

pub fn json_to_string(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

pub fn collect_json(value: &Value) -> CommandOutput {
    CommandOutput::Lines(vec![json_to_string(value)])
}

pub fn collect_table(headers: &[&str], rows: &[Vec<String>]) -> CommandOutput {
    let mut lines = Vec::new();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:width$}", h, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");

    lines.push(header_line.clone());
    lines.push("-".repeat(header_line.len()));

    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:width$}", c, width = widths.get(i).copied().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join("  ");
        lines.push(line);
    }

    CommandOutput::Lines(lines)
}

pub fn collect_single_line(msg: String) -> CommandOutput {
    CommandOutput::Lines(vec![msg])
}

pub struct TreeNode {
    pub title: String,
    pub id: String,
    pub children: Vec<TreeNode>,
}

pub fn collect_tree(header: &str, roots: &[TreeNode]) -> CommandOutput {
    let mut lines = Vec::new();
    lines.push(header.to_string());
    lines.push("─".repeat(header.chars().count()));

    for (i, node) in roots.iter().enumerate() {
        let is_last = i == roots.len() - 1;
        collect_node(&mut lines, node, "", is_last);
    }

    CommandOutput::Lines(lines)
}

fn collect_node(lines: &mut Vec<String>, node: &TreeNode, prefix: &str, is_last: bool) {
    let connector = if is_last { "└─ " } else { "├─ " };
    lines.push(format!(
        "{}{}{} ({})",
        prefix, connector, node.title, node.id
    ));

    let child_prefix = if is_last {
        format!("{}   ", prefix)
    } else {
        format!("{}│  ", prefix)
    };

    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == node.children.len() - 1;
        collect_node(lines, child, &child_prefix, child_is_last);
    }
}
