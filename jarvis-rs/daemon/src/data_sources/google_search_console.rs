//! Google Search Console data source — fetches real SEO performance data.
//!
//! Provides per-page and per-query metrics:
//! - **Clicks**: how many times users clicked a search result
//! - **Impressions**: how many times a page appeared in search results
//! - **CTR**: click-through rate (clicks / impressions)
//! - **Position**: average ranking position in search results
//!
//! These metrics are matched against `daemon_content` by URL and persisted
//! into `daemon_metrics`.

use anyhow::{Context, Result};
use async_trait::async_trait;
use jarvis_daemon_common::{DaemonDb, MetricType};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

use super::google_auth::{self, GoogleOAuthConfig, GoogleTokens};
use super::{DataSource, SyncResult};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the Google Search Console data source.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchConsoleConfig {
    /// The site URL as registered in Search Console.
    /// Usually `https://example.com/` or `sc-domain:example.com`.
    pub site_url: String,
    /// Google OAuth2 client credentials.
    pub oauth: GoogleOAuthConfig,
    /// How many days of data to request (max 16 months, default 30).
    #[serde(default = "default_days")]
    pub days: u32,
    /// Path to the cached Google credentials file.
    /// Defaults to `~/.jarvis/credentials/google.json`.
    #[serde(default)]
    pub credentials_path: Option<PathBuf>,
}

fn default_days() -> u32 {
    30
}

// ---------------------------------------------------------------------------
// API types
// ---------------------------------------------------------------------------

/// Request body for the Search Analytics query endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchAnalyticsRequest {
    start_date: String,
    end_date: String,
    dimensions: Vec<String>,
    row_limit: u32,
}

/// Response from the Search Analytics query endpoint.
#[derive(Debug, Deserialize)]
struct SearchAnalyticsResponse {
    #[serde(default)]
    rows: Vec<SearchAnalyticsRow>,
}

/// A single row of search analytics data.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchAnalyticsRow {
    pub keys: Vec<String>,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client that fetches performance data from Google Search Console.
pub struct SearchConsoleClient {
    config: SearchConsoleConfig,
    http: Client,
}

impl SearchConsoleClient {
    pub fn new(config: SearchConsoleConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { config, http })
    }

    /// Get a valid access token, refreshing if needed.
    async fn get_token(&self) -> Result<GoogleTokens> {
        let creds_path = self
            .config
            .credentials_path
            .clone()
            .unwrap_or_else(google_auth::default_credentials_path);
        google_auth::ensure_valid_token(&self.config.oauth, &creds_path).await
    }

    /// Fetch per-page search analytics for the configured period.
    pub async fn fetch_page_analytics(&self) -> Result<Vec<SearchAnalyticsRow>> {
        let tokens = self.get_token().await?;
        let site_url = &self.config.site_url;

        let end = chrono::Utc::now().date_naive();
        // Search Console data has ~3 day lag
        let end = end - chrono::Duration::days(3);
        let start = end - chrono::Duration::days(self.config.days as i64);

        let url = format!(
            "https://www.googleapis.com/webmasters/v3/sites/{}/searchAnalytics/query",
            urlencoding_encode(site_url)
        );

        let body = SearchAnalyticsRequest {
            start_date: start.format("%Y-%m-%d").to_string(),
            end_date: end.format("%Y-%m-%d").to_string(),
            dimensions: vec!["page".to_string()],
            row_limit: 1000,
        };

        debug!("Fetching Search Console analytics: {url}");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&tokens.access_token)
            .json(&body)
            .send()
            .await
            .context("Search Console request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Search Console returned {status}: {body}");
        }

        let response: SearchAnalyticsResponse = resp
            .json()
            .await
            .context("failed to parse Search Console response")?;

        Ok(response.rows)
    }
}

#[async_trait]
impl DataSource for SearchConsoleClient {
    fn name(&self) -> &str {
        "Google Search Console"
    }

    async fn sync(&self, db: &DaemonDb) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        let rows = match self.fetch_page_analytics().await {
            Ok(r) => r,
            Err(e) => {
                // If credentials are missing, return a helpful error instead of crashing.
                result
                    .errors
                    .push(format!("Search Console fetch failed: {e:#}"));
                return Ok(result);
            }
        };

        info!(
            "Search Console: fetched {} page rows (last {} days)",
            rows.len(),
            self.config.days
        );

        let now = chrono::Utc::now().timestamp();
        let period_start = now - (self.config.days as i64 * 86400);

        for row in &rows {
            let page_url = match row.keys.first() {
                Some(u) => u,
                None => continue,
            };

            // Try to match the URL to daemon_content.
            let content = match db.find_content_by_url(page_url).await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    // Try stripping trailing slash.
                    let alt = page_url.trim_end_matches('/');
                    match db.find_content_by_url(alt).await {
                        Ok(Some(c)) => c,
                        _ => {
                            debug!("No daemon_content match for: {page_url}");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("DB lookup error for {page_url}: {e}"));
                    continue;
                }
            };

            // Persist clicks.
            if row.clicks > 0.0 {
                if let Err(e) = db
                    .insert_metric(
                        &content.pipeline_id,
                        Some(&content.id),
                        MetricType::Clicks,
                        row.clicks,
                        "google_search_console",
                        period_start,
                        now,
                    )
                    .await
                {
                    result.errors.push(format!("clicks metric insert: {e}"));
                } else {
                    result.records_synced += 1;
                }
            }

            // Persist impressions.
            if row.impressions > 0.0 {
                if let Err(e) = db
                    .insert_metric(
                        &content.pipeline_id,
                        Some(&content.id),
                        MetricType::Impressions,
                        row.impressions,
                        "google_search_console",
                        period_start,
                        now,
                    )
                    .await
                {
                    result
                        .errors
                        .push(format!("impressions metric insert: {e}"));
                } else {
                    result.records_synced += 1;
                }
            }

            // Persist CTR.
            if row.ctr > 0.0 {
                if let Err(e) = db
                    .insert_metric(
                        &content.pipeline_id,
                        Some(&content.id),
                        MetricType::Ctr,
                        row.ctr * 100.0, // Store as percentage
                        "google_search_console",
                        period_start,
                        now,
                    )
                    .await
                {
                    result.errors.push(format!("ctr metric insert: {e}"));
                } else {
                    result.records_synced += 1;
                }
            }

            debug!(
                "SC: {} — {:.0} clicks, {:.0} impressions, {:.1}% CTR, pos {:.1}",
                content.title.as_deref().unwrap_or("?"),
                row.clicks,
                row.impressions,
                row.ctr * 100.0,
                row.position,
            );
        }

        info!(
            "Search Console sync complete: {} records synced, {} errors",
            result.records_synced,
            result.errors.len()
        );

        Ok(result)
    }
}

/// Minimal percent-encoding for URL path segments.
fn urlencoding_encode(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_config_minimal() {
        let config: SearchConsoleConfig = serde_json::from_value(serde_json::json!({
            "site_url": "https://example.com/",
            "oauth": {
                "client_id": "id.apps.googleusercontent.com",
                "client_secret": "secret"
            }
        }))
        .expect("parse");
        assert_eq!(config.site_url, "https://example.com/");
        assert_eq!(config.days, 30); // default
        assert!(config.credentials_path.is_none());
    }

    #[test]
    fn parse_config_full() {
        let config: SearchConsoleConfig = serde_json::from_value(serde_json::json!({
            "site_url": "sc-domain:example.com",
            "oauth": {
                "client_id": "id",
                "client_secret": "secret",
                "scopes": ["https://www.googleapis.com/auth/webmasters.readonly"]
            },
            "days": 60,
            "credentials_path": "/tmp/google.json"
        }))
        .expect("parse");
        assert_eq!(config.site_url, "sc-domain:example.com");
        assert_eq!(config.days, 60);
        assert_eq!(
            config.credentials_path.unwrap().to_str().unwrap(),
            "/tmp/google.json"
        );
    }

    #[test]
    fn parse_search_analytics_row() {
        let row: SearchAnalyticsRow = serde_json::from_value(serde_json::json!({
            "keys": ["https://example.com/my-post"],
            "clicks": 42.0,
            "impressions": 1500.0,
            "ctr": 0.028,
            "position": 8.5
        }))
        .expect("parse");
        assert_eq!(row.keys[0], "https://example.com/my-post");
        assert!((row.clicks - 42.0).abs() < f64::EPSILON);
        assert!((row.impressions - 1500.0).abs() < f64::EPSILON);
        assert!((row.ctr - 0.028).abs() < 0.001);
        assert!((row.position - 8.5).abs() < 0.1);
    }

    #[test]
    fn parse_search_analytics_response_empty() {
        let resp: SearchAnalyticsResponse =
            serde_json::from_value(serde_json::json!({})).expect("parse");
        assert!(resp.rows.is_empty());
    }

    #[test]
    fn parse_search_analytics_response_with_rows() {
        let resp: SearchAnalyticsResponse = serde_json::from_value(serde_json::json!({
            "rows": [
                {"keys": ["https://a.com/"], "clicks": 10.0, "impressions": 200.0, "ctr": 0.05, "position": 3.2},
                {"keys": ["https://b.com/"], "clicks": 5.0, "impressions": 100.0, "ctr": 0.05, "position": 7.0}
            ]
        }))
        .expect("parse");
        assert_eq!(resp.rows.len(), 2);
    }

    #[test]
    fn urlencoding_works() {
        assert_eq!(
            urlencoding_encode("https://example.com/"),
            "https%3A%2F%2Fexample.com%2F"
        );
    }

    #[test]
    fn client_construction_succeeds() {
        let config = SearchConsoleConfig {
            site_url: "https://example.com/".to_string(),
            oauth: GoogleOAuthConfig {
                client_id: "id".to_string(),
                client_secret: "secret".to_string(),
                scopes: vec![],
            },
            days: 30,
            credentials_path: None,
        };
        let client = SearchConsoleClient::new(config);
        assert!(client.is_ok());
    }
}
