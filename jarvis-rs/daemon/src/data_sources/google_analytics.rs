//! Google Analytics 4 (GA4) data source — fetches engagement metrics.
//!
//! Provides per-page engagement data:
//! - **Sessions**: number of sessions
//! - **Bounce rate**: percentage of single-page sessions
//! - **Avg session duration**: average time on page (seconds)
//! - **Engaged sessions**: sessions with >10s, or conversion, or 2+ page views
//! - **Page views**: total views per page
//!
//! Uses the GA4 Data API (v1beta) via the same Google OAuth2 flow
//! as Search Console and AdSense.

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::MetricType;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
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

/// Configuration for the Google Analytics 4 data source.
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleAnalyticsConfig {
    /// GA4 property ID (numeric, e.g. "123456789").
    pub property_id: String,
    /// Google OAuth2 client credentials.
    pub oauth: GoogleOAuthConfig,
    /// How many days of data to request (default 30).
    #[serde(default = "default_days")]
    pub days: u32,
    /// Path to cached Google credentials file.
    #[serde(default)]
    pub credentials_path: Option<PathBuf>,
}

fn default_days() -> u32 {
    30
}

// ---------------------------------------------------------------------------
// GA4 Data API types
// ---------------------------------------------------------------------------

/// Request body for GA4 runReport endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ga4ReportRequest {
    date_ranges: Vec<Ga4DateRange>,
    dimensions: Vec<Ga4Dimension>,
    metrics: Vec<Ga4Metric>,
    limit: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ga4DateRange {
    start_date: String,
    end_date: String,
}

#[derive(Debug, Serialize)]
struct Ga4Dimension {
    name: String,
}

#[derive(Debug, Serialize)]
struct Ga4Metric {
    name: String,
}

/// Response from GA4 runReport endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ga4ReportResponse {
    #[serde(default)]
    rows: Vec<Ga4Row>,
    #[serde(default)]
    row_count: Option<i64>,
}

/// A single row from a GA4 report.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ga4Row {
    dimension_values: Vec<Ga4Value>,
    metric_values: Vec<Ga4Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct Ga4Value {
    value: String,
}

/// Parsed per-page engagement data.
#[derive(Debug, Clone)]
pub struct PageEngagement {
    /// The page path (e.g. "/blog/my-article").
    pub page_path: String,
    /// Total sessions for this page.
    pub sessions: i64,
    /// Engaged sessions (sessions with >10s, conversion, or 2+ views).
    pub engaged_sessions: i64,
    /// Total page views.
    pub page_views: i64,
    /// Average session duration in seconds.
    pub avg_session_duration: f64,
    /// Bounce rate (0.0 to 1.0).
    pub bounce_rate: f64,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client that fetches engagement data from Google Analytics 4.
pub struct GoogleAnalyticsClient {
    config: GoogleAnalyticsConfig,
    http: Client,
}

impl GoogleAnalyticsClient {
    pub fn new(config: GoogleAnalyticsConfig) -> Result<Self> {
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

    /// Fetch per-page engagement metrics from GA4.
    pub async fn fetch_page_engagement(&self) -> Result<Vec<PageEngagement>> {
        let tokens = self.get_token().await?;

        let end = chrono::Utc::now().date_naive();
        let start = end - chrono::Duration::days(self.config.days as i64);

        let url = format!(
            "https://analyticsdata.googleapis.com/v1beta/properties/{}:runReport",
            self.config.property_id,
        );

        let body = Ga4ReportRequest {
            date_ranges: vec![Ga4DateRange {
                start_date: start.format("%Y-%m-%d").to_string(),
                end_date: end.format("%Y-%m-%d").to_string(),
            }],
            dimensions: vec![Ga4Dimension {
                name: "pagePath".to_string(),
            }],
            metrics: vec![
                Ga4Metric {
                    name: "sessions".to_string(),
                },
                Ga4Metric {
                    name: "engagedSessions".to_string(),
                },
                Ga4Metric {
                    name: "screenPageViews".to_string(),
                },
                Ga4Metric {
                    name: "averageSessionDuration".to_string(),
                },
                Ga4Metric {
                    name: "bounceRate".to_string(),
                },
            ],
            limit: 1000,
        };

        debug!("Fetching GA4 engagement: {url}");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&tokens.access_token)
            .json(&body)
            .send()
            .await
            .context("GA4 request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("GA4 returned {status}: {body_text}");
        }

        let response: Ga4ReportResponse =
            resp.json().await.context("failed to parse GA4 response")?;

        info!(
            "GA4 returned {} rows (row_count: {:?})",
            response.rows.len(),
            response.row_count
        );

        Ok(parse_engagement_rows(&response.rows))
    }
}

/// Parse GA4 rows into structured PageEngagement records.
fn parse_engagement_rows(rows: &[Ga4Row]) -> Vec<PageEngagement> {
    rows.iter()
        .filter_map(|row| {
            let page_path = row.dimension_values.first()?.value.clone();
            let sessions = parse_int(&row.metric_values, 0);
            let engaged_sessions = parse_int(&row.metric_values, 1);
            let page_views = parse_int(&row.metric_values, 2);
            let avg_session_duration = parse_float(&row.metric_values, 3);
            let bounce_rate = parse_float(&row.metric_values, 4);

            Some(PageEngagement {
                page_path,
                sessions,
                engaged_sessions,
                page_views,
                avg_session_duration,
                bounce_rate,
            })
        })
        .collect()
}

fn parse_int(values: &[Ga4Value], idx: usize) -> i64 {
    values
        .get(idx)
        .and_then(|v| v.value.parse().ok())
        .unwrap_or(0)
}

fn parse_float(values: &[Ga4Value], idx: usize) -> f64 {
    values
        .get(idx)
        .and_then(|v| v.value.parse().ok())
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// DataSource trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl DataSource for GoogleAnalyticsClient {
    fn name(&self) -> &str {
        "Google Analytics 4"
    }

    async fn sync(&self, db: &DaemonDb) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        let engagements = match self.fetch_page_engagement().await {
            Ok(e) => e,
            Err(e) => {
                result.errors.push(format!("GA4 fetch failed: {e:#}"));
                return Ok(result);
            }
        };

        info!(
            "GA4: fetched {} page rows (last {} days)",
            engagements.len(),
            self.config.days,
        );

        let now = chrono::Utc::now().timestamp();
        let period_start = now - (self.config.days as i64 * 86400);

        for page in &engagements {
            // Match page path to content by URL or slug.
            let content = match db.find_content_by_url(&page.page_path).await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    let slug = page
                        .page_path
                        .rsplit('/')
                        .find(|s| !s.is_empty())
                        .unwrap_or("");
                    if slug.is_empty() {
                        continue;
                    }
                    match db.find_content_by_slug(slug).await {
                        Ok(Some(c)) => c,
                        _ => {
                            debug!("No daemon_content match for GA4: {}", page.page_path);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("DB lookup error for {}: {e}", page.page_path));
                    continue;
                }
            };

            // Record page views.
            if page.page_views > 0 {
                if let Err(e) = db
                    .insert_metric(
                        &content.pipeline_id,
                        Some(&content.id),
                        MetricType::Views,
                        page.page_views as f64,
                        "google_analytics",
                        period_start,
                        now,
                    )
                    .await
                {
                    result.errors.push(format!("GA4 views metric: {e}"));
                } else {
                    result.records_synced += 1;
                }
            }

            // Record impressions (using sessions as proxy for GA4 impressions).
            if page.sessions > 0 {
                if let Err(e) = db
                    .insert_metric(
                        &content.pipeline_id,
                        Some(&content.id),
                        MetricType::Impressions,
                        page.sessions as f64,
                        "google_analytics",
                        period_start,
                        now,
                    )
                    .await
                {
                    result.errors.push(format!("GA4 sessions metric: {e}"));
                } else {
                    result.records_synced += 1;
                }
            }
        }

        info!(
            "GA4 sync: {} records synced, {} errors",
            result.records_synced,
            result.errors.len(),
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

    fn fake_oauth() -> GoogleOAuthConfig {
        GoogleOAuthConfig {
            client_id: "test-id".to_string(),
            client_secret: "test-secret".to_string(),
            scopes: vec!["https://www.googleapis.com/auth/analytics.readonly".to_string()],
        }
    }

    #[test]
    fn parse_config_minimal() {
        let config: GoogleAnalyticsConfig = serde_json::from_value(serde_json::json!({
            "property_id": "123456789",
            "oauth": {
                "client_id": "id",
                "client_secret": "secret"
            }
        }))
        .expect("parse");
        assert_eq!(config.property_id, "123456789");
        assert_eq!(config.days, 30);
    }

    #[test]
    fn parse_config_full() {
        let config: GoogleAnalyticsConfig = serde_json::from_value(serde_json::json!({
            "property_id": "987654321",
            "oauth": {
                "client_id": "id",
                "client_secret": "secret",
                "scopes": ["https://www.googleapis.com/auth/analytics.readonly"]
            },
            "days": 60,
            "credentials_path": "/tmp/ga-creds.json"
        }))
        .expect("parse");
        assert_eq!(config.property_id, "987654321");
        assert_eq!(config.days, 60);
        assert_eq!(
            config.credentials_path.unwrap().to_str().unwrap(),
            "/tmp/ga-creds.json"
        );
    }

    #[test]
    fn client_construction_succeeds() {
        let config = GoogleAnalyticsConfig {
            property_id: "123".to_string(),
            oauth: fake_oauth(),
            days: 30,
            credentials_path: None,
        };
        GoogleAnalyticsClient::new(config).expect("should construct");
    }

    #[test]
    fn parse_ga4_response_empty() {
        let resp: Ga4ReportResponse = serde_json::from_value(serde_json::json!({
            "rows": [],
            "rowCount": 0
        }))
        .expect("parse");
        assert!(resp.rows.is_empty());
    }

    #[test]
    fn parse_ga4_response_with_rows() {
        let resp: Ga4ReportResponse = serde_json::from_value(serde_json::json!({
            "rows": [
                {
                    "dimensionValues": [{"value": "/blog/test-article"}],
                    "metricValues": [
                        {"value": "150"},
                        {"value": "120"},
                        {"value": "200"},
                        {"value": "45.5"},
                        {"value": "0.25"}
                    ]
                }
            ],
            "rowCount": 1
        }))
        .expect("parse");

        let engagements = parse_engagement_rows(&resp.rows);
        assert_eq!(engagements.len(), 1);

        let p = &engagements[0];
        assert_eq!(p.page_path, "/blog/test-article");
        assert_eq!(p.sessions, 150);
        assert_eq!(p.engaged_sessions, 120);
        assert_eq!(p.page_views, 200);
        assert!((p.avg_session_duration - 45.5).abs() < 0.01);
        assert!((p.bounce_rate - 0.25).abs() < 0.01);
    }

    #[test]
    fn parse_engagement_handles_missing_metrics() {
        let row = Ga4Row {
            dimension_values: vec![Ga4Value {
                value: "/test".to_string(),
            }],
            metric_values: vec![], // No metrics
        };
        let engagements = parse_engagement_rows(&[row]);
        assert_eq!(engagements.len(), 1);
        assert_eq!(engagements[0].sessions, 0);
        assert_eq!(engagements[0].page_views, 0);
    }
}
