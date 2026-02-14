//! RSS/Atom feed scraper.

use super::Scraper;
use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{DaemonSource, ScrapedContent};
use sha2::{Digest, Sha256};

/// Scraper for RSS and Atom feeds.
pub struct RssScraper {
    http: reqwest::Client,
}

impl RssScraper {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("JarvisDaemon/1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Parse RSS/Atom XML into scraped content items.
    fn parse_feed(xml: &str, source_url: &str) -> Result<Vec<ScrapedContent>> {
        // Simple XML parsing: extract <item> or <entry> elements.
        // In production, use the `feed-rs` crate for robust parsing.
        // For now, we use a basic regex-based approach.
        let mut items = Vec::new();

        // Try RSS <item> format first.
        let item_regex = regex_lite::Regex::new(r"(?s)<item>(.*?)</item>")
            .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;

        let title_regex =
            regex_lite::Regex::new(r"(?s)<title>(?:<!\[CDATA\[)?(.*?)(?:\]\]>)?</title>")
                .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;

        let desc_regex = regex_lite::Regex::new(
            r"(?s)<description>(?:<!\[CDATA\[)?(.*?)(?:\]\]>)?</description>",
        )
        .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;

        let link_regex = regex_lite::Regex::new(r"<link>(?:<!\[CDATA\[)?(.*?)(?:\]\]>)?</link>")
            .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;

        for cap in item_regex.captures_iter(xml) {
            let item_xml = &cap[1];

            let title = title_regex
                .captures(item_xml)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default();

            let body = desc_regex
                .captures(item_xml)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_default();

            let url = link_regex
                .captures(item_xml)
                .map(|c| c[1].trim().to_string())
                .unwrap_or_else(|| source_url.to_string());

            if title.is_empty() {
                continue;
            }

            let content_hash = compute_hash(&format!("{title}{body}"));

            items.push(ScrapedContent {
                title,
                body,
                url,
                content_hash,
                metadata: serde_json::json!({"source_type": "rss"}),
            });
        }

        // If no RSS items found, try Atom <entry> format.
        if items.is_empty() {
            let entry_regex = regex_lite::Regex::new(r"(?s)<entry>(.*?)</entry>")
                .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;

            let atom_link_regex = regex_lite::Regex::new(r#"<link[^>]*href="([^"]*)"[^>]*/>"#)
                .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;

            let summary_regex = regex_lite::Regex::new(r"(?s)<summary[^>]*>(.*?)</summary>")
                .map_err(|e| anyhow::anyhow!("regex error: {e}"))?;

            for cap in entry_regex.captures_iter(xml) {
                let entry_xml = &cap[1];

                let title = title_regex
                    .captures(entry_xml)
                    .map(|c| c[1].trim().to_string())
                    .unwrap_or_default();

                let body = summary_regex
                    .captures(entry_xml)
                    .or_else(|| desc_regex.captures(entry_xml))
                    .map(|c| c[1].trim().to_string())
                    .unwrap_or_default();

                let url = atom_link_regex
                    .captures(entry_xml)
                    .map(|c| c[1].trim().to_string())
                    .unwrap_or_else(|| source_url.to_string());

                if title.is_empty() {
                    continue;
                }

                let content_hash = compute_hash(&format!("{title}{body}"));

                items.push(ScrapedContent {
                    title,
                    body,
                    url,
                    content_hash,
                    metadata: serde_json::json!({"source_type": "atom"}),
                });
            }
        }

        Ok(items)
    }
}

#[async_trait]
impl Scraper for RssScraper {
    fn source_type(&self) -> &str {
        "rss"
    }

    async fn scrape(&self, source: &DaemonSource) -> Result<Vec<ScrapedContent>> {
        let resp = self.http.get(&source.url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("RSS fetch failed with status {}", resp.status());
        }
        let xml = resp.text().await?;
        Self::parse_feed(&xml, &source.url)
    }

    async fn check_for_updates(&self, source: &DaemonSource) -> Result<Option<String>> {
        let resp = self.http.get(&source.url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("RSS fetch failed with status {}", resp.status());
        }
        let xml = resp.text().await?;
        let new_hash = compute_hash(&xml);

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
    fn parse_rss_feed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
    <title>Test Feed</title>
    <item>
        <title>First Article</title>
        <description>Description of the first article.</description>
        <link>https://example.com/article-1</link>
    </item>
    <item>
        <title>Second Article</title>
        <description><![CDATA[<p>HTML description</p>]]></description>
        <link>https://example.com/article-2</link>
    </item>
</channel>
</rss>"#;

        let items = RssScraper::parse_feed(xml, "https://example.com/feed").expect("parse feed");

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "First Article");
        assert_eq!(items[0].url, "https://example.com/article-1");
        assert_eq!(items[1].title, "Second Article");
    }

    #[test]
    fn parse_atom_feed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
    <title>Atom Feed</title>
    <entry>
        <title>Atom Entry 1</title>
        <link href="https://example.com/entry-1" rel="alternate"/>
        <summary>Summary of entry 1.</summary>
    </entry>
</feed>"#;

        let items = RssScraper::parse_feed(xml, "https://example.com/feed").expect("parse feed");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Atom Entry 1");
    }
}
