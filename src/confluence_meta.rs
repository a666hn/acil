use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMeta {
    pub page_id: String,
    pub title: String,
    pub space: String,
    pub version: i64,
    pub url: String,
}

const FRONTMATTER_DELIMITER: &str = "---";

pub fn parse(content: &str) -> Result<(PageMeta, &str)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with(FRONTMATTER_DELIMITER) {
        return Err(AppError::Config(
            "Missing frontmatter (expected ---) ".into(),
        ));
    }

    let after_first = &trimmed[FRONTMATTER_DELIMITER.len()..];
    let end = after_first
        .find(FRONTMATTER_DELIMITER)
        .ok_or_else(|| AppError::Config("Unclosed frontmatter (missing closing ---)".into()))?;

    let yaml_str = &after_first[..end];
    let meta: PageMeta =
        serde_yaml::from_str(yaml_str).map_err(|e| AppError::Config(e.to_string()))?;

    let body_start = end + FRONTMATTER_DELIMITER.len();
    let body = after_first[body_start..].trim_start_matches('\n');

    Ok((meta, body))
}

impl PageMeta {
    pub fn to_frontmatter(&self) -> String {
        let yaml = serde_yaml::to_string(self).unwrap_or_default();
        format!(
            "{}\n{}{}\n",
            FRONTMATTER_DELIMITER, yaml, FRONTMATTER_DELIMITER
        )
    }

    pub fn build_url(base_url: &str, page_id: &str) -> String {
        let base = base_url.trim_end_matches('/');
        format!("{}/wiki/pages/viewpage.action?pageId={}", base, page_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let input = r#"---
page_id: "123"
title: "Test Page"
space: "PROJ"
version: 5
url: "https://example.atlassian.net/wiki/pages/viewpage.action?pageId=123"
---

# Hello

Body content here"#;

        let (meta, body) = parse(input).unwrap();
        assert_eq!(meta.page_id, "123");
        assert_eq!(meta.title, "Test Page");
        assert_eq!(meta.space, "PROJ");
        assert_eq!(meta.version, 5);
        assert!(body.starts_with("# Hello"));
        assert!(body.contains("Body content here"));
    }

    #[test]
    fn test_to_frontmatter() {
        let meta = PageMeta {
            page_id: "456".into(),
            title: "My Page".into(),
            space: "DEV".into(),
            version: 3,
            url: "https://example.atlassian.net/wiki/pages/viewpage.action?pageId=456".into(),
        };
        let fm = meta.to_frontmatter();
        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---\n"));
        assert!(fm.contains("456"));
        assert!(fm.contains("My Page"));
        assert!(fm.contains("page_id"));
    }

    #[test]
    fn test_roundtrip() {
        let meta = PageMeta {
            page_id: "789".into(),
            title: "Roundtrip".into(),
            space: "TEST".into(),
            version: 1,
            url: "https://x.atlassian.net/wiki/pages/viewpage.action?pageId=789".into(),
        };
        let body = "# Content\n\nSome text.";
        let file = format!("{}\n{}", meta.to_frontmatter(), body);
        let (parsed_meta, parsed_body) = parse(&file).unwrap();
        assert_eq!(parsed_meta.page_id, "789");
        assert_eq!(parsed_meta.version, 1);
        assert!(parsed_body.starts_with("# Content"));
    }

    #[test]
    fn test_build_url() {
        let url = PageMeta::build_url("https://my-domain.atlassian.net", "12345");
        assert_eq!(
            url,
            "https://my-domain.atlassian.net/wiki/pages/viewpage.action?pageId=12345"
        );
    }
}
