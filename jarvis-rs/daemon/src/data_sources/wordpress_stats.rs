//! WordPress Stats data source — fetches real pageview data from WordPress sites.
//!
//! Supports two modes:
//!
//! 1. **WP Statistics plugin** — REST endpoint at `/wp-json/wp-statistics/v2/`
//! 2. **WordPress.com / Jetpack Stats** — via `stats.wordpress.com`
//!
//! The collected pageviews are matched against `daemon_content` by URL or slug
//! and persisted as real metrics in the `daemon_metrics` table.

use anyhow::{Context, Result};
use async_trait::async_trait;
use jarvis_daemon_common::{DaemonDb, MetricType};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info, warn};

use super::{DataSource, SyncResult};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Which WordPress stats plugin / method to use.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatsPlugin {
    /// WP Statistics plugin REST API.
    WpStatistics,
    /// Jetpack / WordPress.com Stats REST API.
    Jetpack,
}

impl Default for StatsPlugin {
    fn default() -> Self {
        Self::WpStatistics
    }
}

/// Configuration for the WordPress Stats data source.
#[derive(Debug, Clone, Deserialize)]
pub struct WordPressStatsConfig {
    /// WordPress site base URL (e.g. `https://my-blog.com`).
    pub base_url: String,
    /// Authentication type.
    #[serde(default)]
    pub auth: WordPressAuth,
    /// Which stats plugin to query.
    #[serde(default)]
    pub stats_plugin: StatsPlugin,
    /// How many days of data to pull.
    #[serde(default = "default_sync_days")]
    pub sync_days: u32,
}

fn default_sync_days() -> u32 {
    30
}

/// Authentication for the WordPress REST API.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WordPressAuth {
    /// Application Password authentication.
    ApplicationPassword {
        username: String,
        /// The password. If absent, read from `WORDPRESS_APP_PASSWORD` env var.
        password: Option<String>,
    },
    /// Jetpack API key (for WordPress.com stats).
    JetpackApiKey {
        /// The API key. If absent, read from `JETPACK_API_KEY` env var.
        api_key: Option<String>,
    },
    /// No authentication (for public endpoints).
    None,
}

impl Default for WordPressAuth {
    fn default() -> Self {
        Self::None
    }
}

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

/// A single post's view stats from WP Statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct WpStatisticsPostView {
    #[serde(alias = "post_id")]
    pub id: i64,
    #[serde(alias = "post_title")]
    pub title: Option<String>,
    #[serde(alias = "post_link", alias = "permalink")]
    pub link: Option<String>,
    #[serde(alias = "hits", alias = "views")]
    pub count: i64,
}

/// A single post from the WP REST API `/wp/v2/posts`.
#[derive(Debug, Clone, Deserialize)]
pub struct WpPost {
    pub id: i64,
    pub slug: String,
    pub link: String,
    pub title: WpRendered,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WpRendered {
    pub rendered: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client that fetches stats from a WordPress site.
pub struct WordPressStatsClient {
    config: WordPressStatsConfig,
    http: Client,
}

impl WordPressStatsClient {
    pub fn new(config: WordPressStatsConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { config, http })
    }

    /// Build a request with the configured authentication.
    fn authenticated_get(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.http.get(url);
        match &self.config.auth {
            WordPressAuth::ApplicationPassword { username, password } => {
                let pw = password
                    .clone()
                    .or_else(|| std::env::var("WORDPRESS_APP_PASSWORD").ok())
                    .unwrap_or_default();
                req = req.basic_auth(username, Some(&pw));
            }
            WordPressAuth::JetpackApiKey { api_key } => {
                let key = api_key
                    .clone()
                    .or_else(|| std::env::var("JETPACK_API_KEY").ok())
                    .unwrap_or_default();
                req = req.bearer_auth(&key);
            }
            WordPressAuth::None => {}
        }
        req
    }

    /// Fetch per-post view stats using the WP Statistics plugin REST API.
    async fn fetch_wp_statistics_views(&self) -> Result<Vec<WpStatisticsPostView>> {
        let base = self.config.base_url.trim_end_matches('/');
        let url = format!(
            "{base}/wp-json/wp-statistics/v2/posts?per_page=100&days={}",
            self.config.sync_days
        );

        debug!("Fetching WP Statistics views: {url}");
        let resp = self
            .authenticated_get(&url)
            .send()
            .await
            .context("WP Statistics request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("WP Statistics returned {status}: {body}");
        }

        let views: Vec<WpStatisticsPostView> = resp
            .json()
            .await
            .context("failed to parse WP Statistics response")?;

        Ok(views)
    }

    /// Fetch recent posts via the standard WP REST API to get URL ↔ slug mapping.
    async fn fetch_recent_posts(&self, per_page: u32) -> Result<Vec<WpPost>> {
        let base = self.config.base_url.trim_end_matches('/');
        let url = format!("{base}/wp-json/wp/v2/posts?per_page={per_page}&orderby=date&order=desc");

        debug!("Fetching WP posts: {url}");
        let resp = self
            .authenticated_get(&url)
            .send()
            .await
            .context("WP posts request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("WP posts API returned {status}: {body}");
        }

        let posts: Vec<WpPost> = resp
            .json()
            .await
            .context("failed to parse WP posts response")?;

        Ok(posts)
    }

    /// Match a WordPress post view record to a `daemon_content` entry by URL or slug.
    async fn match_content(
        &self,
        db: &DaemonDb,
        view: &WpStatisticsPostView,
        posts_map: &std::collections::HashMap<i64, WpPost>,
    ) -> Option<jarvis_daemon_common::DaemonContent> {
        // Try matching by the view's link first.
        if let Some(link) = &view.link {
            if let Ok(Some(content)) = db.find_content_by_url(link).await {
                return Some(content);
            }
        }

        // Fallback: use the post's slug from the posts list.
        if let Some(post) = posts_map.get(&view.id) {
            if let Ok(Some(content)) = db.find_content_by_slug(&post.slug).await {
                return Some(content);
            }
            // Try matching by post link.
            if let Ok(Some(content)) = db.find_content_by_url(&post.link).await {
                return Some(content);
            }
        }

        None
    }
}

#[async_trait]
impl DataSource for WordPressStatsClient {
    fn name(&self) -> &str {
        "WordPress Stats"
    }

    async fn sync(&self, db: &DaemonDb) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        // 1. Fetch view stats.
        let views = match self.config.stats_plugin {
            StatsPlugin::WpStatistics => self.fetch_wp_statistics_views().await?,
            StatsPlugin::Jetpack => {
                // Jetpack support is Phase 3 — skip for now.
                warn!("Jetpack Stats integration not yet implemented, skipping");
                return Ok(result);
            }
        };

        info!(
            "WordPress Stats: fetched {} post view records (last {} days)",
            views.len(),
            self.config.sync_days
        );

        if views.is_empty() {
            return Ok(result);
        }

        // 2. Fetch posts mapping for slug/URL lookup.
        let posts = self.fetch_recent_posts(100).await.unwrap_or_else(|e| {
            result
                .errors
                .push(format!("failed to fetch WP posts list: {e}"));
            Vec::new()
        });
        let posts_map: std::collections::HashMap<i64, WpPost> =
            posts.into_iter().map(|p| (p.id, p)).collect();

        // 3. Match each view record to daemon_content and persist metrics.
        let now = chrono::Utc::now().timestamp();
        let period_start = now - (self.config.sync_days as i64 * 86400);

        for view in &views {
            if view.count <= 0 {
                continue;
            }

            match self.match_content(db, view, &posts_map).await {
                Some(content) => {
                    if let Err(e) = db
                        .insert_metric(
                            &content.pipeline_id,
                            Some(&content.id),
                            MetricType::Views,
                            view.count as f64,
                            "wordpress_stats",
                            period_start,
                            now,
                        )
                        .await
                    {
                        result
                            .errors
                            .push(format!("failed to insert metric for '{}': {e}", content.id));
                    } else {
                        result.records_synced += 1;
                        debug!(
                            "Synced {} views for content '{}' ({})",
                            view.count,
                            content.title.as_deref().unwrap_or("untitled"),
                            content.id
                        );
                    }
                }
                None => {
                    debug!(
                        "No matching daemon_content for WP post id={}, title={:?}",
                        view.id, view.title
                    );
                }
            }
        }

        info!(
            "WordPress Stats sync complete: {} records synced, {} errors",
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
    fn parse_config_defaults() {
        let config: WordPressStatsConfig = serde_json::from_value(serde_json::json!({
            "base_url": "https://example.com"
        }))
        .expect("parse");

        assert_eq!(config.base_url, "https://example.com");
        assert_eq!(config.sync_days, 30);
        assert_eq!(config.stats_plugin, StatsPlugin::WpStatistics);
    }

    #[test]
    fn parse_config_with_auth() {
        let config: WordPressStatsConfig = serde_json::from_value(serde_json::json!({
            "base_url": "https://my-blog.com",
            "auth": {
                "type": "application_password",
                "username": "admin",
                "password": "secret-app-pass"
            },
            "stats_plugin": "wp_statistics",
            "sync_days": 14
        }))
        .expect("parse");

        assert_eq!(config.base_url, "https://my-blog.com");
        assert_eq!(config.sync_days, 14);
        assert!(matches!(
            config.auth,
            WordPressAuth::ApplicationPassword {
                ref username,
                ..
            } if username == "admin"
        ));
    }

    #[test]
    fn parse_config_jetpack() {
        let config: WordPressStatsConfig = serde_json::from_value(serde_json::json!({
            "base_url": "https://example.wordpress.com",
            "auth": {
                "type": "jetpack_api_key",
                "api_key": "my-jetpack-key"
            },
            "stats_plugin": "jetpack"
        }))
        .expect("parse");

        assert_eq!(config.stats_plugin, StatsPlugin::Jetpack);
        assert!(matches!(config.auth, WordPressAuth::JetpackApiKey { .. }));
    }

    #[test]
    fn parse_wp_statistics_post_view() {
        let json = serde_json::json!({
            "id": 42,
            "post_title": "How to pass concursos",
            "post_link": "https://blog.com/how-to-pass-concursos",
            "hits": 350
        });
        let view: WpStatisticsPostView = serde_json::from_value(json).expect("parse");
        assert_eq!(view.id, 42);
        assert_eq!(view.count, 350);
        assert_eq!(
            view.link.as_deref(),
            Some("https://blog.com/how-to-pass-concursos")
        );
    }

    #[test]
    fn parse_wp_statistics_post_view_alternate_fields() {
        let json = serde_json::json!({
            "post_id": 99,
            "permalink": "https://blog.com/outro-post",
            "views": 120
        });
        let view: WpStatisticsPostView = serde_json::from_value(json).expect("parse");
        assert_eq!(view.id, 99);
        assert_eq!(view.count, 120);
    }

    #[test]
    fn parse_wp_post() {
        let json = serde_json::json!({
            "id": 42,
            "slug": "how-to-pass-concursos",
            "link": "https://blog.com/how-to-pass-concursos/",
            "title": {"rendered": "How to Pass Concursos"}
        });
        let post: WpPost = serde_json::from_value(json).expect("parse");
        assert_eq!(post.id, 42);
        assert_eq!(post.slug, "how-to-pass-concursos");
        assert_eq!(post.title.rendered, "How to Pass Concursos");
    }

    #[tokio::test]
    async fn sync_result_default() {
        let result = SyncResult::default();
        assert_eq!(result.records_synced, 0);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn client_construction_succeeds() {
        let config = WordPressStatsConfig {
            base_url: "https://example.com".to_string(),
            auth: WordPressAuth::None,
            stats_plugin: StatsPlugin::WpStatistics,
            sync_days: 7,
        };
        let client = WordPressStatsClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn jetpack_sync_returns_early() {
        let config = WordPressStatsConfig {
            base_url: "https://example.com".to_string(),
            auth: WordPressAuth::None,
            stats_plugin: StatsPlugin::Jetpack,
            sync_days: 7,
        };
        let client = WordPressStatsClient::new(config).unwrap();
        let db = DaemonDb::open_memory().await.expect("open db");
        let result = client.sync(&db).await.expect("sync");
        // Jetpack is not yet implemented, should return empty result.
        assert_eq!(result.records_synced, 0);
    }
}
