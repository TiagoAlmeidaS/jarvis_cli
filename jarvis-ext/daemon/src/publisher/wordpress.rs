//! WordPress REST API publisher.
//!
//! Uses the WordPress Application Passwords authentication method.
//! Docs: https://developer.wordpress.org/rest-api/reference/posts/

use super::Publisher;
use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::ContentOutput;
use jarvis_daemon_common::Platform;
use jarvis_daemon_common::PublishResult;
use serde::Deserialize;

/// Configuration for WordPress publishing.
#[derive(Debug, Clone, Deserialize)]
pub struct WordPressConfig {
    /// Base URL of the WordPress site (e.g., "https://myblog.com").
    pub base_url: String,
    /// WordPress username.
    pub username: String,
    /// Application password (or env var WORDPRESS_APP_PASSWORD).
    pub password: Option<String>,
    /// Category IDs to assign to posts.
    #[serde(default)]
    pub category_ids: Vec<i64>,
    /// Default post status: "publish" or "draft".
    #[serde(default = "default_status")]
    pub default_status: String,
}

fn default_status() -> String {
    "publish".to_string()
}

pub struct WordPressPublisher {
    http: reqwest::Client,
    config: WordPressConfig,
    password: String,
}

impl WordPressPublisher {
    /// Create from pipeline publisher config.
    pub fn from_config(config: &serde_json::Value) -> Result<Self> {
        let wp_config: WordPressConfig = serde_json::from_value(
            config
                .get("publisher")
                .cloned()
                .unwrap_or(serde_json::json!({})),
        )?;

        let password = wp_config
            .password
            .clone()
            .or_else(|| std::env::var("WORDPRESS_APP_PASSWORD").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "WordPress password must be set in config or WORDPRESS_APP_PASSWORD env var"
                )
            })?;

        Ok(Self {
            http: reqwest::Client::new(),
            config: wp_config,
            password,
        })
    }
}

#[async_trait]
impl Publisher for WordPressPublisher {
    fn platform(&self) -> Platform {
        Platform::Wordpress
    }

    async fn publish(&self, content: &ContentOutput) -> Result<PublishResult> {
        let url = format!("{}/wp-json/wp/v2/posts", self.config.base_url);

        let body = serde_json::json!({
            "title": content.title,
            "slug": content.slug,
            "content": content.body,
            "status": self.config.default_status,
            "categories": self.config.category_ids,
        });

        let resp = self
            .http
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.password))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("WordPress API error {status}: {text}");
        }

        let result: serde_json::Value = resp.json().await?;

        let post_url = result["link"].as_str().unwrap_or("").to_string();
        let post_id = result["id"].as_i64().map(|id| id.to_string());

        Ok(PublishResult {
            external_url: post_url,
            external_id: post_id,
            platform: Platform::Wordpress,
        })
    }
}
