//! Google AdSense data source — fetches real revenue data per page.
//!
//! Provides:
//! - **Estimated Earnings**: actual revenue per page
//! - **Page Views**: AdSense-tracked page views
//! - **Page Views RPM**: revenue per 1000 page views
//!
//! Revenue is matched against `daemon_content` by URL and persisted
//! into `daemon_revenue` (source = `adsense`).

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::CreateRevenue;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::RevenueSource;
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::debug;
use tracing::info;

use super::DataSource;
use super::SyncResult;
use super::google_auth::GoogleOAuthConfig;
use super::google_auth::GoogleTokens;
use super::google_auth::{self};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the Google AdSense data source.
#[derive(Debug, Clone, Deserialize)]
pub struct AdSenseConfig {
    /// AdSense account ID (e.g. `accounts/pub-1234567890`).
    pub account_id: String,
    /// Google OAuth2 client credentials (shared with Search Console).
    pub oauth: GoogleOAuthConfig,
    /// How many days of data to request (default 30).
    #[serde(default = "default_days")]
    pub days: u32,
    /// Path to the cached Google credentials file.
    #[serde(default)]
    pub credentials_path: Option<PathBuf>,
}

fn default_days() -> u32 {
    30
}

// ---------------------------------------------------------------------------
// API types
// ---------------------------------------------------------------------------

/// Response from the AdSense reports endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdSenseReportResponse {
    #[serde(default)]
    rows: Vec<AdSenseRow>,
    #[serde(default)]
    headers: Vec<AdSenseHeader>,
    #[serde(default)]
    total_matched_rows: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdSenseHeader {
    name: String,
    #[serde(rename = "type")]
    header_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdSenseRow {
    cells: Vec<AdSenseCell>,
}

#[derive(Debug, Deserialize)]
struct AdSenseCell {
    value: Option<String>,
}

/// Parsed AdSense row with typed fields.
#[derive(Debug, Clone)]
pub struct AdSensePageReport {
    pub page_url: String,
    pub estimated_earnings: f64,
    pub page_views: f64,
    pub page_views_rpm: f64,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client that fetches revenue data from Google AdSense.
pub struct AdSenseClient {
    config: AdSenseConfig,
    http: Client,
}

impl AdSenseClient {
    pub fn new(config: AdSenseConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { config, http })
    }

    async fn get_token(&self) -> Result<GoogleTokens> {
        let creds_path = self
            .config
            .credentials_path
            .clone()
            .unwrap_or_else(google_auth::default_credentials_path);
        google_auth::ensure_valid_token(&self.config.oauth, &creds_path).await
    }

    /// Fetch per-page revenue report from AdSense.
    pub async fn fetch_page_report(&self) -> Result<Vec<AdSensePageReport>> {
        let tokens = self.get_token().await?;

        let end = chrono::Utc::now().date_naive();
        let start = end - chrono::Duration::days(self.config.days as i64);

        let url = format!(
            "https://adsense.googleapis.com/v2/{}/reports:generate",
            self.config.account_id
        );

        debug!("Fetching AdSense report: {url}");
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&tokens.access_token)
            .query(&[
                ("dateRange.startDate.year", start.format("%Y").to_string()),
                ("dateRange.startDate.month", start.format("%-m").to_string()),
                ("dateRange.startDate.day", start.format("%-d").to_string()),
                ("dateRange.endDate.year", end.format("%Y").to_string()),
                ("dateRange.endDate.month", end.format("%-m").to_string()),
                ("dateRange.endDate.day", end.format("%-d").to_string()),
            ])
            .query(&[
                ("metrics", "ESTIMATED_EARNINGS"),
                ("metrics", "PAGE_VIEWS"),
                ("metrics", "PAGE_VIEWS_RPM"),
            ])
            .query(&[("dimensions", "PAGE")])
            .query(&[("currencyCode", "USD")])
            .send()
            .await
            .context("AdSense request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("AdSense returned {status}: {body}");
        }

        let report: AdSenseReportResponse = resp
            .json()
            .await
            .context("failed to parse AdSense response")?;

        // Parse the tabular response into typed structs.
        // Columns: PAGE, ESTIMATED_EARNINGS, PAGE_VIEWS, PAGE_VIEWS_RPM
        let pages: Vec<AdSensePageReport> = report
            .rows
            .iter()
            .filter_map(|row| {
                let cells = &row.cells;
                if cells.len() < 4 {
                    return None;
                }
                let page_url = cells[0].value.clone().unwrap_or_default();
                let earnings = cells[1]
                    .value
                    .as_deref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let views = cells[2]
                    .value
                    .as_deref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let rpm = cells[3]
                    .value
                    .as_deref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0);

                if page_url.is_empty() {
                    return None;
                }

                Some(AdSensePageReport {
                    page_url,
                    estimated_earnings: earnings,
                    page_views: views,
                    page_views_rpm: rpm,
                })
            })
            .collect();

        Ok(pages)
    }
}

#[async_trait]
impl DataSource for AdSenseClient {
    fn name(&self) -> &str {
        "Google AdSense"
    }

    async fn sync(&self, db: &DaemonDb) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        let pages = match self.fetch_page_report().await {
            Ok(p) => p,
            Err(e) => {
                result.errors.push(format!("AdSense fetch failed: {e:#}"));
                return Ok(result);
            }
        };

        info!(
            "AdSense: fetched {} page reports (last {} days)",
            pages.len(),
            self.config.days
        );

        let now = chrono::Utc::now().timestamp();
        let period_start = now - (self.config.days as i64 * 86400);

        for page in &pages {
            if page.estimated_earnings <= 0.0 {
                continue;
            }

            // Try to match the URL to daemon_content.
            let content = match db.find_content_by_url(&page.page_url).await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    let alt = page.page_url.trim_end_matches('/');
                    match db.find_content_by_url(alt).await {
                        Ok(Some(c)) => c,
                        _ => {
                            debug!("No daemon_content match for: {}", page.page_url);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("DB lookup error for {}: {e}", page.page_url));
                    continue;
                }
            };

            // Persist revenue.
            let rev_input = CreateRevenue {
                content_id: Some(content.id.clone()),
                pipeline_id: content.pipeline_id.clone(),
                source: RevenueSource::Adsense,
                amount: page.estimated_earnings,
                currency: Some("USD".to_string()),
                period_start,
                period_end: now,
                external_id: None,
                metadata: Some(serde_json::json!({
                    "page_url": page.page_url,
                    "page_views": page.page_views,
                    "page_views_rpm": page.page_views_rpm,
                    "source": "google_adsense",
                    "days": self.config.days,
                })),
            };

            if let Err(e) = db.create_revenue(&rev_input).await {
                result
                    .errors
                    .push(format!("revenue insert for '{}': {e}", content.id));
            } else {
                result.records_synced += 1;
                debug!(
                    "AdSense: ${:.4} from '{}' ({:.0} views, ${:.2} RPM)",
                    page.estimated_earnings,
                    content.title.as_deref().unwrap_or("?"),
                    page.page_views,
                    page.page_views_rpm,
                );
            }
        }

        info!(
            "AdSense sync complete: {} records synced, {} errors",
            result.records_synced,
            result.errors.len()
        );

        Ok(result)
    }
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
        let config: AdSenseConfig = serde_json::from_value(serde_json::json!({
            "account_id": "accounts/pub-123456",
            "oauth": {
                "client_id": "id.apps.googleusercontent.com",
                "client_secret": "secret"
            }
        }))
        .expect("parse");
        assert_eq!(config.account_id, "accounts/pub-123456");
        assert_eq!(config.days, 30);
    }

    #[test]
    fn parse_config_full() {
        let config: AdSenseConfig = serde_json::from_value(serde_json::json!({
            "account_id": "accounts/pub-999",
            "oauth": {
                "client_id": "id",
                "client_secret": "secret",
                "scopes": ["https://www.googleapis.com/auth/adsense.readonly"]
            },
            "days": 7,
            "credentials_path": "/tmp/creds.json"
        }))
        .expect("parse");
        assert_eq!(config.days, 7);
        assert!(config.credentials_path.is_some());
    }

    #[test]
    fn parse_adsense_report_response_empty() {
        let resp: AdSenseReportResponse =
            serde_json::from_value(serde_json::json!({})).expect("parse");
        assert!(resp.rows.is_empty());
    }

    #[test]
    fn parse_adsense_report_response_with_rows() {
        let resp: AdSenseReportResponse = serde_json::from_value(serde_json::json!({
            "headers": [
                {"name": "PAGE", "type": "DIMENSION"},
                {"name": "ESTIMATED_EARNINGS", "type": "METRIC"},
                {"name": "PAGE_VIEWS", "type": "METRIC"},
                {"name": "PAGE_VIEWS_RPM", "type": "METRIC"}
            ],
            "rows": [
                {"cells": [
                    {"value": "https://blog.com/post-1"},
                    {"value": "1.25"},
                    {"value": "350"},
                    {"value": "3.57"}
                ]},
                {"cells": [
                    {"value": "https://blog.com/post-2"},
                    {"value": "0.50"},
                    {"value": "120"},
                    {"value": "4.17"}
                ]}
            ]
        }))
        .expect("parse");
        assert_eq!(resp.rows.len(), 2);
        assert_eq!(
            resp.rows[0].cells[0].value.as_deref(),
            Some("https://blog.com/post-1")
        );
    }

    #[test]
    fn parse_page_report_from_rows() {
        let row = AdSenseRow {
            cells: vec![
                AdSenseCell {
                    value: Some("https://blog.com/my-post".to_string()),
                },
                AdSenseCell {
                    value: Some("2.50".to_string()),
                },
                AdSenseCell {
                    value: Some("500".to_string()),
                },
                AdSenseCell {
                    value: Some("5.00".to_string()),
                },
            ],
        };
        let cells = &row.cells;
        let report = AdSensePageReport {
            page_url: cells[0].value.clone().unwrap_or_default(),
            estimated_earnings: cells[1]
                .value
                .as_deref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            page_views: cells[2]
                .value
                .as_deref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            page_views_rpm: cells[3]
                .value
                .as_deref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
        };

        assert_eq!(report.page_url, "https://blog.com/my-post");
        assert!((report.estimated_earnings - 2.50).abs() < 0.01);
        assert!((report.page_views - 500.0).abs() < f64::EPSILON);
        assert!((report.page_views_rpm - 5.00).abs() < 0.01);
    }

    #[test]
    fn client_construction_succeeds() {
        let config = AdSenseConfig {
            account_id: "accounts/pub-123".to_string(),
            oauth: GoogleOAuthConfig {
                client_id: "id".to_string(),
                client_secret: "secret".to_string(),
                scopes: vec![],
            },
            days: 30,
            credentials_path: None,
        };
        let client = AdSenseClient::new(config);
        assert!(client.is_ok());
    }
}
