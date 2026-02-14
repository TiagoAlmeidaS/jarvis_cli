//! Generic web scraper for HTML pages.

use super::Scraper;
use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{DaemonSource, ScrapedContent};
use sha2::{Digest, Sha256};

/// Basic web scraper that fetches HTML and extracts text.
pub struct WebScraper {
    http: reqwest::Client,
}

impl WebScraper {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("JarvisDaemon/1.0 (compatible; bot)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Strip HTML tags and normalize whitespace.
    fn strip_html(html: &str) -> String {
        // These regexes are compile-time constants and cannot fail.
        let tag_re = regex_lite::Regex::new(r"<[^>]+>").unwrap_or_else(|_| unreachable!());
        let text = tag_re.replace_all(html, " ");
        let ws_re = regex_lite::Regex::new(r"\s+").unwrap_or_else(|_| unreachable!());
        ws_re.replace_all(&text, " ").trim().to_string()
    }

    /// Extract content matching a CSS-like selector.
    /// This is a simplified version; for production, use the `scraper` crate.
    /// Currently supports basic class selectors like ".class-name".
    fn extract_with_selector(html: &str, selector: &str) -> Vec<String> {
        let mut results = Vec::new();

        // Handle simple class selector.
        if let Some(class) = selector.strip_prefix('.') {
            let pattern = format!(r#"(?s)class="[^"]*\b{class}\b[^"]*"[^>]*>(.*?)</[^>]+>"#);
            if let Ok(re) = regex_lite::Regex::new(&pattern) {
                for cap in re.captures_iter(html) {
                    let inner = &cap[1];
                    let text = Self::strip_html(inner);
                    if !text.is_empty() {
                        results.push(text);
                    }
                }
            }
        }

        // If no selector matches, return the whole body text.
        if results.is_empty() {
            let body_text = Self::strip_html(html);
            if !body_text.is_empty() {
                results.push(body_text);
            }
        }

        results
    }
}

#[async_trait]
impl Scraper for WebScraper {
    fn source_type(&self) -> &str {
        "webpage"
    }

    async fn scrape(&self, source: &DaemonSource) -> Result<Vec<ScrapedContent>> {
        let resp = self.http.get(&source.url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Web fetch failed with status {}", resp.status());
        }
        let html = resp.text().await?;

        let sections = if let Some(ref selector) = source.scrape_selector {
            Self::extract_with_selector(&html, selector)
        } else {
            vec![Self::strip_html(&html)]
        };

        let items: Vec<ScrapedContent> = sections
            .into_iter()
            .enumerate()
            .map(|(i, body)| {
                let title = body.chars().take(80).collect::<String>();
                let content_hash = compute_hash(&body);
                ScrapedContent {
                    title: if title.len() < body.len() {
                        format!("{title}...")
                    } else {
                        title
                    },
                    body,
                    url: format!("{}#{i}", source.url),
                    content_hash,
                    metadata: serde_json::json!({
                        "source_type": "webpage",
                        "selector": source.scrape_selector,
                    }),
                }
            })
            .collect();

        Ok(items)
    }

    async fn check_for_updates(&self, source: &DaemonSource) -> Result<Option<String>> {
        let resp = self.http.get(&source.url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Web fetch failed with status {}", resp.status());
        }
        let html = resp.text().await?;
        let new_hash = compute_hash(&html);

        if let Some(ref old_hash) = source.last_content_hash
            && *old_hash == new_hash
        {
            return Ok(None);
        }

        Ok(Some(new_hash))
    }
}

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_basic() {
        let html = "<p>Hello <strong>world</strong></p>";
        let text = WebScraper::strip_html(html);
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn extract_with_class_selector() {
        let html = r#"<div class="item active">First item</div><div class="item">Second item</div><div class="other">Not this</div>"#;
        let results = WebScraper::extract_with_selector(html, ".item");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "First item");
        assert_eq!(results[1], "Second item");
    }
}
