//! Scraper adapters for fetching content from external sources.

pub mod rss;
pub mod web;

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::DaemonSource;
use jarvis_daemon_common::ScrapedContent;

/// Trait for scraping content from a data source.
#[async_trait]
pub trait Scraper: Send + Sync {
    /// The source type this scraper handles (e.g., "rss", "webpage").
    fn source_type(&self) -> &str;

    /// Scrape content from the given source.
    /// Returns a list of new content items found.
    async fn scrape(&self, source: &DaemonSource) -> Result<Vec<ScrapedContent>>;

    /// Check if the source has new content since the last check.
    /// Returns the new content hash if changed, None if unchanged.
    async fn check_for_updates(&self, source: &DaemonSource) -> Result<Option<String>>;
}

/// Get the appropriate scraper for a source type.
pub fn get_scraper(source_type: &str) -> Option<Box<dyn Scraper>> {
    match source_type {
        "rss" => Some(Box::new(rss::RssScraper::new())),
        "webpage" => Some(Box::new(web::WebScraper::new())),
        _ => None,
    }
}
