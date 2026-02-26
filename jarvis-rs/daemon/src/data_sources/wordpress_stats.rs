//! WordPress Stats data source — fetches real pageview data from WordPress sites.
//!
//! Supports two modes:
//!
//! 1. **WP Statistics plugin** — REST endpoint at `/wp-json/wp-statistics/v2/`
//! 2. **WordPress.com / Jetpack Stats** — via `stats.wordpress.com`
//!
//! The collected pageviews are matched against `daemon_content` by URL or slug
//! and persisted as real metrics in the `daemon_metrics` table.

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::MetricType;
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;
use tracing::info;

use super::DataSource;
use super::SyncResult;

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

// ---------------------------------------------------------------------------
// Jetpack Stats API response types
// ---------------------------------------------------------------------------

/// Response from the Jetpack Stats REST API `/wpcom/v2/stats/posts`.
#[derive(Debug, Clone, Deserialize)]
pub struct JetpackStatsResponse {
    /// Per-day views for the site.
    #[serde(default)]
    pub posts: Vec<JetpackPostStats>,
}

/// A single post's stats from Jetpack / WordPress.com.
#[derive(Debug, Clone, Deserialize)]
pub struct JetpackPostStats {
    /// Post ID.
    #[serde(alias = "ID")]
    pub id: i64,
    /// Post title.
    pub title: Option<String>,
    /// Post URL.
    #[serde(alias = "URL")]
    pub url: Option<String>,
    /// Total views for this post in the period.
    pub views: i64,
}

/// Response from the Jetpack top-posts endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct JetpackTopPostsResponse {
    /// Summary stats.
    #[serde(default)]
    pub summary: Option<JetpackSummary>,
    /// Per-day data.
    #[serde(default)]
    pub days: std::collections::HashMap<String, JetpackDayData>,
}

/// Daily data from Jetpack.
#[derive(Debug, Clone, Deserialize)]
pub struct JetpackDayData {
    /// Top posts for the day.
    #[serde(default)]
    pub postviews: Vec<JetpackPostView>,
    /// Total views for the day.
    #[serde(default)]
    pub total_views: Option<i64>,
}

/// A single post view entry from Jetpack top-posts.
#[derive(Debug, Clone, Deserialize)]
pub struct JetpackPostView {
    pub id: i64,
    pub title: Option<String>,
    #[serde(alias = "href")]
    pub url: Option<String>,
    pub views: i64,
}

/// Jetpack summary stats.
#[derive(Debug, Clone, Deserialize)]
pub struct JetpackSummary {
    pub views: Option<i64>,
    pub visitors: Option<i64>,
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

    /// Fetch per-post view stats using the Jetpack / WordPress.com Stats REST API.
    ///
    /// Uses two approaches:
    /// 1. The top-posts endpoint (`/wp-json/wpcom/v2/stats/top-posts`) for per-post views.
    /// 2. Falls back to the site-level stats endpoint if available.
    async fn fetch_jetpack_views(&self) -> Result<Vec<WpStatisticsPostView>> {
        let base = self.config.base_url.trim_end_matches('/');

        // Try the Jetpack REST API endpoint for top posts.
        let url = format!(
            "{base}/wp-json/wpcom/v2/stats/top-posts?num={}&period=day",
            self.config.sync_days
        );

        debug!("Fetching Jetpack top-posts: {url}");
        let resp = self
            .authenticated_get(&url)
            .send()
            .await
            .context("Jetpack Stats request failed")?;

        if !resp.status().is_success() {
            // Fallback: try the WordPress.com REST API v1.1 format.
            return self.fetch_jetpack_views_wpcom().await;
        }

        let top_posts: JetpackTopPostsResponse = resp
            .json()
            .await
            .context("failed to parse Jetpack top-posts response")?;

        // Aggregate views per post across all days.
        let mut post_views: std::collections::HashMap<i64, (Option<String>, Option<String>, i64)> =
            std::collections::HashMap::new();

        for (_date, day_data) in &top_posts.days {
            for pv in &day_data.postviews {
                let entry = post_views
                    .entry(pv.id)
                    .or_insert_with(|| (pv.title.clone(), pv.url.clone(), 0));
                entry.2 += pv.views;
            }
        }

        let views = post_views
            .into_iter()
            .map(|(id, (title, url, count))| WpStatisticsPostView {
                id,
                title,
                link: url,
                count,
            })
            .collect();

        Ok(views)
    }

    /// Fallback: Fetch Jetpack stats via WordPress.com REST API v1.1 format.
    ///
    /// Endpoint: `https://public-api.wordpress.com/rest/v1.1/sites/{site}/stats/top-posts`
    async fn fetch_jetpack_views_wpcom(&self) -> Result<Vec<WpStatisticsPostView>> {
        // Extract site identifier (domain or site ID) from base_url.
        let site = self
            .config
            .base_url
            .trim_end_matches('/')
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        let url = format!(
            "https://public-api.wordpress.com/rest/v1.1/sites/{site}/stats/top-posts?num={}&period=day",
            self.config.sync_days,
        );

        debug!("Fetching Jetpack stats via WP.com API: {url}");
        let resp = self
            .authenticated_get(&url)
            .send()
            .await
            .context("Jetpack WP.com stats request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Jetpack WP.com API returned {status}: {body}");
        }

        let top_posts: JetpackTopPostsResponse = resp
            .json()
            .await
            .context("failed to parse Jetpack WP.com stats response")?;

        let mut post_views: std::collections::HashMap<i64, (Option<String>, Option<String>, i64)> =
            std::collections::HashMap::new();

        for (_date, day_data) in &top_posts.days {
            for pv in &day_data.postviews {
                let entry = post_views
                    .entry(pv.id)
                    .or_insert_with(|| (pv.title.clone(), pv.url.clone(), 0));
                entry.2 += pv.views;
            }
        }

        let views = post_views
            .into_iter()
            .map(|(id, (title, url, count))| WpStatisticsPostView {
                id,
                title,
                link: url,
                count,
            })
            .collect();

        Ok(views)
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
            StatsPlugin::Jetpack => self.fetch_jetpack_views().await?,
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
                    let source_name = match self.config.stats_plugin {
                        StatsPlugin::WpStatistics => "wordpress_stats",
                        StatsPlugin::Jetpack => "jetpack_stats",
                    };
                    if let Err(e) = db
                        .insert_metric(
                            &content.pipeline_id,
                            Some(&content.id),
                            MetricType::Views,
                            view.count as f64,
                            source_name,
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

    #[test]
    fn parse_jetpack_top_posts_response() {
        let json = serde_json::json!({
            "summary": {"views": 500, "visitors": 200},
            "days": {
                "2026-02-10": {
                    "postviews": [
                        {"id": 1, "title": "Post A", "href": "https://blog.com/post-a", "views": 100},
                        {"id": 2, "title": "Post B", "href": "https://blog.com/post-b", "views": 50}
                    ],
                    "total_views": 150
                },
                "2026-02-11": {
                    "postviews": [
                        {"id": 1, "title": "Post A", "href": "https://blog.com/post-a", "views": 80},
                        {"id": 3, "title": "Post C", "href": "https://blog.com/post-c", "views": 30}
                    ],
                    "total_views": 110
                }
            }
        });
        let resp: JetpackTopPostsResponse = serde_json::from_value(json).expect("parse");
        assert_eq!(resp.days.len(), 2);

        let summary = resp.summary.as_ref().expect("summary");
        assert_eq!(summary.views, Some(500));
        assert_eq!(summary.visitors, Some(200));

        let day = &resp.days["2026-02-10"];
        assert_eq!(day.postviews.len(), 2);
        assert_eq!(day.postviews[0].views, 100);
    }

    #[test]
    fn parse_jetpack_post_view() {
        let json = serde_json::json!({
            "id": 42,
            "title": "Test Article",
            "href": "https://blog.com/test-article",
            "views": 250
        });
        let pv: JetpackPostView = serde_json::from_value(json).expect("parse");
        assert_eq!(pv.id, 42);
        assert_eq!(pv.views, 250);
        assert_eq!(pv.url.as_deref(), Some("https://blog.com/test-article"));
    }

    #[test]
    fn parse_jetpack_response_empty_days() {
        let json = serde_json::json!({
            "days": {}
        });
        let resp: JetpackTopPostsResponse = serde_json::from_value(json).expect("parse");
        assert!(resp.days.is_empty());
        assert!(resp.summary.is_none());
    }

    #[test]
    fn parse_jetpack_stats_response() {
        let json = serde_json::json!({
            "posts": [
                {"ID": 1, "title": "First", "URL": "https://blog.com/first", "views": 100},
                {"ID": 2, "title": "Second", "URL": "https://blog.com/second", "views": 50}
            ]
        });
        let resp: JetpackStatsResponse = serde_json::from_value(json).expect("parse");
        assert_eq!(resp.posts.len(), 2);
        assert_eq!(resp.posts[0].id, 1);
        assert_eq!(resp.posts[0].views, 100);
    }
}
