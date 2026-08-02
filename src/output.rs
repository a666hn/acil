use std::io::{self, IsTerminal, Write};
use std::sync::OnceLock;

use nu_ansi_term::{Color, Style};
use serde_json::Value;

fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal())
}

pub fn paint(text: &str, style: Style) -> String {
    if color_enabled() {
        style.paint(text).to_string()
    } else {
        text.to_string()
    }
}

pub fn bold(text: &str) -> String {
    paint(text, Style::new().bold())
}

pub fn dim(text: &str) -> String {
    paint(text, Style::new().fg(Color::DarkGray))
}

fn visible_width(s: &str) -> usize {
    let mut width = 0usize;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

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
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(visible_width(cell));
            }
        }
    }

    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:width$}", h, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");
    let total_width = widths.iter().sum::<usize>() + 2 * widths.len().saturating_sub(1);

    lines.push(bold(&header_line));
    lines.push(dim(&"-".repeat(total_width)));

    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let width = widths.get(i).copied().unwrap_or(0);
                let pad = width.saturating_sub(visible_width(c));
                format!("{}{}", c, " ".repeat(pad))
            })
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
    lines.push(bold(header));
    lines.push(dim(&"─".repeat(header.chars().count())));

    for (i, node) in roots.iter().enumerate() {
        let is_last = i == roots.len() - 1;
        collect_node(&mut lines, node, "", is_last);
    }

    CommandOutput::Lines(lines)
}

fn collect_node(lines: &mut Vec<String>, node: &TreeNode, prefix: &str, is_last: bool) {
    let connector = if is_last { "└─ " } else { "├─ " };
    lines.push(format!(
        "{}{}{} {}",
        prefix,
        dim(connector),
        node.title,
        dim(&format!("({})", node.id))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn test_collect_table_aligns_columns_despite_ansi_codes() {
        // "Bug" wrapped in ANSI color codes has more bytes than plain "Story",
        // but fewer visible characters — padding must be based on visible width.
        let colored_short = "\u{1b}[31mBug\u{1b}[0m".to_string();
        let plain_long = "Story".to_string();
        let rows = vec![
            vec![colored_short, "Q".to_string()],
            vec![plain_long, "Q".to_string()],
        ];

        let out = collect_table(&["Type", "V"], &rows);
        let lines = match out {
            CommandOutput::Lines(lines) => lines,
            CommandOutput::Empty => panic!("expected lines"),
        };

        let row0 = strip_ansi(&lines[2]);
        let row1 = strip_ansi(&lines[3]);

        assert_eq!(row0.find('Q'), row1.find('Q'), "columns must line up");
    }
}
