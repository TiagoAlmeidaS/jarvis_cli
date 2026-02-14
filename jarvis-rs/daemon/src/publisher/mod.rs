//! Publisher adapters for posting content to external platforms.

pub mod wordpress;

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{ContentOutput, Platform, PublishResult};

/// Trait for publishing content to an external platform.
#[async_trait]
pub trait Publisher: Send + Sync {
    /// The platform this publisher targets.
    fn platform(&self) -> Platform;

    /// Publish content and return the result (URL, external ID).
    async fn publish(&self, content: &ContentOutput) -> Result<PublishResult>;
}

/// Get the appropriate publisher for a platform.
pub fn get_publisher(platform: Platform, config: &serde_json::Value) -> Result<Box<dyn Publisher>> {
    match platform {
        Platform::Wordpress => {
            let wp = wordpress::WordPressPublisher::from_config(config)?;
            Ok(Box::new(wp))
        }
        other => anyhow::bail!("publisher not yet implemented for platform: {other}"),
    }
}
