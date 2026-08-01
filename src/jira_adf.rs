use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde_json::{Value, json};

struct ListState {
    is_ordered: bool,
    items: Vec<Value>,
    current_item_content: Vec<Value>,
}

pub fn markdown_to_adf(md: &str) -> Value {
    let parser = Parser::new_ext(md, Options::all());
    let mut content: Vec<Value> = Vec::new();
    let mut inline_buf: Vec<Value> = Vec::new();
    let mut marks: Vec<Value> = Vec::new();
    let mut list_stack: Vec<ListState> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                    list_stack.push(ListState {
                        is_ordered: false,
                        items: Vec::new(),
                        current_item_content: vec![json!({
                            "type": "heading",
                            "attrs": { "level": level as u32 },
                            "content": []
                        })],
                    });
                }
                Tag::Paragraph => {
                    flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                }
                Tag::Strong => marks.push(json!({ "type": "strong" })),
                Tag::Emphasis => marks.push(json!({ "type": "em" })),
                Tag::Strikethrough => marks.push(json!({ "type": "strike" })),
                Tag::Link { dest_url, .. } => marks.push(json!({
                    "type": "link",
                    "attrs": { "href": dest_url.to_string() }
                })),
                Tag::CodeBlock(kind) => {
                    flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                    in_code_block = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(info) => info.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code_buf.clear();
                }
                Tag::List(Some(_)) => {
                    flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                    list_stack.push(ListState {
                        is_ordered: true,
                        items: Vec::new(),
                        current_item_content: Vec::new(),
                    });
                }
                Tag::List(None) => {
                    flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                    list_stack.push(ListState {
                        is_ordered: false,
                        items: Vec::new(),
                        current_item_content: Vec::new(),
                    });
                }
                Tag::Item => {
                    flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                    if let Some(state) = list_stack.last_mut() {
                        state.current_item_content = Vec::new();
                    }
                }
                Tag::BlockQuote(_) => {
                    flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                    list_stack.push(ListState {
                        is_ordered: false,
                        items: Vec::new(),
                        current_item_content: Vec::new(),
                    });
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    if let Some(state) = list_stack.pop()
                        && let Some(mut heading) = state.current_item_content.first().cloned()
                    {
                        heading["content"] = json!(std::mem::take(&mut inline_buf));
                        if list_stack.is_empty() {
                            content.push(heading);
                        } else if let Some(parent) = list_stack.last_mut() {
                            parent.current_item_content.push(heading);
                        }
                    }
                }
                TagEnd::Paragraph => {
                    let para = json!({
                        "type": "paragraph",
                        "content": std::mem::take(&mut inline_buf)
                    });
                    if let Some(state) = list_stack.last_mut() {
                        state.current_item_content.push(para);
                    } else {
                        content.push(para);
                    }
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough | TagEnd::Link => {
                    marks.pop();
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let mut attrs = json!({});
                    if !code_lang.is_empty() {
                        attrs["language"] = json!(code_lang);
                    }
                    content.push(json!({
                        "type": "codeBlock",
                        "attrs": attrs,
                        "content": [{ "type": "text", "text": code_buf }]
                    }));
                    code_buf.clear();
                }
                TagEnd::List(_) => {
                    if let Some(state) = list_stack.pop() {
                        let list_type = if state.is_ordered {
                            "orderedList"
                        } else {
                            "bulletList"
                        };
                        let list_node = json!({
                            "type": list_type,
                            "content": state.items
                        });
                        if let Some(parent) = list_stack.last_mut() {
                            parent.current_item_content.push(list_node);
                        } else {
                            content.push(list_node);
                        }
                    }
                }
                TagEnd::Item => {
                    if let Some(state) = list_stack.last_mut() {
                        let item = json!({
                            "type": "listItem",
                            "content": std::mem::take(&mut state.current_item_content)
                        });
                        state.items.push(item);
                    }
                }
                TagEnd::BlockQuote(_) => {
                    if let Some(state) = list_stack.pop() {
                        let bq = json!({
                            "type": "blockquote",
                            "content": state.current_item_content
                        });
                        if let Some(parent) = list_stack.last_mut() {
                            parent.current_item_content.push(bq);
                        } else {
                            content.push(bq);
                        }
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_buf.push_str(&text);
                } else {
                    push_text(&mut inline_buf, &text, &marks);
                }
            }
            Event::Code(code) => {
                let mut code_marks = marks.clone();
                code_marks.push(json!({ "type": "code" }));
                push_text(&mut inline_buf, &code, &code_marks);
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    code_buf.push('\n');
                } else {
                    inline_buf.push(json!({ "type": "hardBreak" }));
                }
            }
            Event::Rule => {
                flush_inline(&mut inline_buf, &mut list_stack, &mut content);
                content.push(json!({ "type": "rule" }));
            }
            _ => {}
        }
    }

    flush_inline(&mut inline_buf, &mut list_stack, &mut content);

    json!({
        "version": 1,
        "type": "doc",
        "content": content
    })
}

fn push_text(buf: &mut Vec<Value>, text: &str, marks: &[Value]) {
    let mut node = json!({ "type": "text", "text": text });
    if !marks.is_empty() {
        node["marks"] = json!(marks);
    }
    buf.push(node);
}

fn flush_inline(inline: &mut Vec<Value>, list_stack: &mut [ListState], content: &mut Vec<Value>) {
    if inline.is_empty() {
        return;
    }
    let taken = std::mem::take(inline);
    if let Some(state) = list_stack.last_mut() {
        state.current_item_content.push(json!(taken));
    } else {
        content.push(json!(taken));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph() {
        let adf = markdown_to_adf("Hello world");
        assert_eq!(adf["type"], "doc");
        assert_eq!(adf["version"], 1);
        assert_eq!(adf["content"].as_array().unwrap().len(), 1);
        assert_eq!(adf["content"][0]["type"], "paragraph");
        assert_eq!(adf["content"][0]["content"][0]["text"], "Hello world");
    }

    #[test]
    fn test_heading() {
        let adf = markdown_to_adf("## My Title");
        assert_eq!(adf["content"][0]["type"], "heading");
        assert_eq!(adf["content"][0]["attrs"]["level"], 2);
        assert_eq!(adf["content"][0]["content"][0]["text"], "My Title");
    }

    #[test]
    fn test_bold() {
        let adf = markdown_to_adf("This is **bold** text");
        let para = &adf["content"][0]["content"];
        assert_eq!(para[0]["text"], "This is ");
        assert_eq!(para[1]["text"], "bold");
        assert_eq!(para[1]["marks"][0]["type"], "strong");
        assert_eq!(para[2]["text"], " text");
    }

    #[test]
    fn test_italic() {
        let adf = markdown_to_adf("This is *italic* text");
        let para = &adf["content"][0]["content"];
        assert_eq!(para[1]["text"], "italic");
        assert_eq!(para[1]["marks"][0]["type"], "em");
    }

    #[test]
    fn test_code_inline() {
        let adf = markdown_to_adf("Use `foo` here");
        let para = &adf["content"][0]["content"];
        assert_eq!(para[1]["text"], "foo");
        assert_eq!(para[1]["marks"][0]["type"], "code");
    }

    #[test]
    fn test_link() {
        let adf = markdown_to_adf("[click](https://example.com)");
        let para = &adf["content"][0]["content"];
        assert_eq!(para[0]["text"], "click");
        assert_eq!(para[0]["marks"][0]["type"], "link");
        assert_eq!(para[0]["marks"][0]["attrs"]["href"], "https://example.com");
    }

    #[test]
    fn test_bullet_list() {
        let adf = markdown_to_adf("- a\n- b\n- c");
        assert_eq!(adf["content"][0]["type"], "bulletList");
        let items = &adf["content"][0]["content"];
        assert_eq!(items.as_array().unwrap().len(), 3);
        assert_eq!(items[0]["type"], "listItem");
    }

    #[test]
    fn test_ordered_list() {
        let adf = markdown_to_adf("1. a\n2. b");
        assert_eq!(adf["content"][0]["type"], "orderedList");
    }

    #[test]
    fn test_code_block() {
        let adf = markdown_to_adf("```rust\nfn main() {}\n```");
        assert_eq!(adf["content"][0]["type"], "codeBlock");
        assert_eq!(adf["content"][0]["attrs"]["language"], "rust");
        assert_eq!(adf["content"][0]["content"][0]["text"], "fn main() {}\n");
    }

    #[test]
    fn test_blockquote() {
        let adf = markdown_to_adf("> quoted text");
        assert_eq!(adf["content"][0]["type"], "blockquote");
    }

    #[test]
    fn test_rule() {
        let adf = markdown_to_adf("---");
        assert_eq!(adf["content"][0]["type"], "rule");
    }

    #[test]
    fn test_multiple_blocks() {
        let adf = markdown_to_adf("# Title\n\nParagraph here");
        assert_eq!(adf["content"].as_array().unwrap().len(), 2);
        assert_eq!(adf["content"][0]["type"], "heading");
        assert_eq!(adf["content"][1]["type"], "paragraph");
    }
}
