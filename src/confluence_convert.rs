use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn markdown_to_storage(md: &str) -> String {
    let parser = Parser::new_ext(md, Options::all());
    let mut out = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();
    let mut list_stack: Vec<bool> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    out.push_str(&format!("<h{}>", level as u32));
                }
                Tag::Paragraph => {
                    out.push_str("<p>");
                }
                Tag::Strong => out.push_str("<strong>"),
                Tag::Emphasis => out.push_str("<em>"),
                Tag::Strikethrough => out.push_str("<del>"),
                Tag::Link { dest_url, .. } => {
                    out.push_str(&format!("<a href=\"{}\">", escape_attr(&dest_url)));
                }
                Tag::Image { dest_url, .. } => {
                    out.push_str(&format!(
                        "<ac:image><ri:url ri:value=\"{}\"/></ac:image>",
                        escape_attr(&dest_url)
                    ));
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(info) => info.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code_buf.clear();
                }
                Tag::List(Some(start)) => {
                    list_stack.push(true);
                    out.push_str(&format!("<ol start=\"{}\">", start));
                }
                Tag::List(None) => {
                    list_stack.push(false);
                    out.push_str("<ul>");
                }
                Tag::Item => out.push_str("<li>"),
                Tag::BlockQuote(_) => out.push_str("<blockquote>"),
                Tag::Table(_) => out.push_str("<table>"),
                Tag::TableHead => out.push_str("<thead><tr>"),
                Tag::TableRow => out.push_str("<tr>"),
                Tag::TableCell => out.push_str("<td>"),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(level) => {
                    out.push_str(&format!("</h{}>\n", level as u32));
                }
                TagEnd::Paragraph => out.push_str("</p>\n"),
                TagEnd::Strong => out.push_str("</strong>"),
                TagEnd::Emphasis => out.push_str("</em>"),
                TagEnd::Strikethrough => out.push_str("</del>"),
                TagEnd::Link => out.push_str("</a>"),
                TagEnd::Image => {}
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    if code_lang.is_empty() {
                        out.push_str(&format!(
                            "<ac:structured-macro ac:name=\"code\"><ac:plain-text-body><![CDATA[{}]]></ac:plain-text-body></ac:structured-macro>\n",
                            code_buf
                        ));
                    } else {
                        out.push_str(&format!(
                            "<ac:structured-macro ac:name=\"code\"><ac:parameter ac:name=\"language\">{}</ac:parameter><ac:plain-text-body><![CDATA[{}]]></ac:plain-text-body></ac:structured-macro>\n",
                            escape_xml(&code_lang),
                            code_buf
                        ));
                    }
                    code_buf.clear();
                }
                TagEnd::List(_) => {
                    if let Some(is_ordered) = list_stack.pop() {
                        if is_ordered {
                            out.push_str("</ol>\n");
                        } else {
                            out.push_str("</ul>\n");
                        }
                    }
                }
                TagEnd::Item => out.push_str("</li>\n"),
                TagEnd::BlockQuote(_) => out.push_str("</blockquote>\n"),
                TagEnd::Table => out.push_str("</table>\n"),
                TagEnd::TableHead => out.push_str("</tr></thead>\n"),
                TagEnd::TableRow => out.push_str("</tr>\n"),
                TagEnd::TableCell => out.push_str("</td>"),
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_buf.push_str(&text);
                } else {
                    out.push_str(&escape_xml(&text));
                }
            }
            Event::Code(code) => {
                out.push_str(&format!("<code>{}</code>", escape_xml(&code)));
            }
            Event::Html(html) => {
                out.push_str(&html);
            }
            Event::InlineHtml(html) => {
                out.push_str(&html);
            }
            Event::SoftBreak => {
                if in_code_block {
                    code_buf.push('\n');
                } else {
                    out.push('\n');
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    code_buf.push('\n');
                } else {
                    out.push_str("<br/>\n");
                }
            }
            Event::Rule => out.push_str("<hr/>\n"),
            _ => {}
        }
    }

    out
}

pub fn storage_to_markdown(xhtml: &str) -> String {
    let converter = htmd::HtmlToMarkdown::builder().build();
    converter
        .convert(xhtml)
        .unwrap_or_else(|_| xhtml.to_string())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading() {
        let result = markdown_to_storage("# Hello");
        assert!(result.contains("<h1>"));
        assert!(result.contains("Hello"));
        assert!(result.contains("</h1>"));
    }

    #[test]
    fn test_bold_italic() {
        let result = markdown_to_storage("**bold** and *italic*");
        assert!(result.contains("<strong>bold</strong>"));
        assert!(result.contains("<em>italic</em>"));
    }

    #[test]
    fn test_link() {
        let result = markdown_to_storage("[click here](https://example.com)");
        assert!(result.contains("<a href=\"https://example.com\">"));
        assert!(result.contains("click here"));
    }

    #[test]
    fn test_code_inline() {
        let result = markdown_to_storage("use `foo` here");
        assert!(result.contains("<code>foo</code>"));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let result = markdown_to_storage(md);
        assert!(result.contains("ac:structured-macro"));
        assert!(result.contains("ac:name=\"code\""));
        assert!(result.contains("fn main() {}"));
        assert!(result.contains("language"));
        assert!(result.contains("rust"));
    }

    #[test]
    fn test_code_block_no_lang() {
        let md = "```\nhello\n```";
        let result = markdown_to_storage(md);
        assert!(result.contains("ac:structured-macro"));
        assert!(result.contains("hello"));
        assert!(!result.contains("language"));
    }

    #[test]
    fn test_list_unordered() {
        let md = "- a\n- b\n- c";
        let result = markdown_to_storage(md);
        assert!(result.contains("<ul>"));
        assert!(result.contains("<li>"));
    }

    #[test]
    fn test_list_ordered() {
        let md = "1. a\n2. b";
        let result = markdown_to_storage(md);
        assert!(result.contains("<ol"));
        assert!(result.contains("<li>"));
    }

    #[test]
    fn test_image() {
        let result = markdown_to_storage("![alt text](https://img.com/pic.png)");
        assert!(result.contains("ac:image"));
        assert!(result.contains("ri:url"));
        assert!(result.contains("https://img.com/pic.png"));
    }

    #[test]
    fn test_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let result = markdown_to_storage(md);
        assert!(result.contains("<table>"));
        assert!(result.contains("<td>"));
    }

    #[test]
    fn test_blockquote() {
        let result = markdown_to_storage("> quoted text");
        assert!(result.contains("<blockquote>"));
        assert!(result.contains("quoted text"));
    }

    #[test]
    fn test_storage_to_markdown_basic() {
        let xhtml = "<h1>Title</h1><p>Hello <strong>world</strong></p>";
        let md = storage_to_markdown(xhtml);
        assert!(md.contains("# Title"));
        assert!(md.contains("**world**"));
    }

    #[test]
    fn test_storage_to_markdown_links() {
        let xhtml = "<p><a href=\"https://example.com\">click</a></p>";
        let md = storage_to_markdown(xhtml);
        assert!(md.contains("[click](https://example.com)"));
    }

    #[test]
    fn test_roundtrip_heading() {
        let md = "# Hello World\n";
        let storage = markdown_to_storage(md);
        let back = storage_to_markdown(&storage);
        assert!(back.contains("# Hello World"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let md = "This is **bold** text.\n";
        let storage = markdown_to_storage(md);
        let back = storage_to_markdown(&storage);
        assert!(back.contains("**bold**"));
    }
}
