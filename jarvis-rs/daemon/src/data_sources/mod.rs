//! External data sources for real metrics and analytics.
//!
//! This module provides a [`DataSource`] trait and concrete implementations
//! that pull real data from external services (WordPress Stats, Google
//! Search Console, AdSense, etc.) into the daemon's SQLite database.
//!
//! The metrics collector pipeline invokes registered data sources on each
//! run so that the strategy analyzer works with real data instead of
//! CPC-based estimates.

pub mod google_adsense;
pub mod google_analytics;
pub mod google_auth;
pub mod google_search_console;
pub mod wordpress_stats;

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::DaemonDb;
use std::sync::Arc;

/// Result of a single data source synchronisation run.
#[derive(Debug, Default)]
pub struct SyncResult {
    /// Number of metric / revenue records written to the database.
    pub records_synced: u64,
    /// Non-fatal errors encountered during sync (individual post failures, etc.).
    pub errors: Vec<String>,
}

/// A pluggable external data source that can push real metrics into the daemon DB.
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Human-readable name for logging (e.g. "WordPress Stats").
    fn name(&self) -> &str;

    /// Pull data from the external source and persist it.
    async fn sync(&self, db: &DaemonDb) -> Result<SyncResult>;
}

/// Registry of enabled data sources.
///
/// The metrics collector iterates over all registered sources on each tick.
pub struct DataSourceRegistry {
    sources: Vec<Arc<dyn DataSource>>,
}

impl DataSourceRegistry {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Register a data source.
    pub fn register(&mut self, source: Arc<dyn DataSource>) {
        self.sources.push(source);
    }

    /// Return an iterator over all registered data sources.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn DataSource>> {
        self.sources.iter()
    }

    /// Number of registered sources.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Whether any sources are registered.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

impl Default for DataSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    struct FakeSource {
        name: String,
        records: u64,
    }

    #[async_trait]
    impl DataSource for FakeSource {
        fn name(&self) -> &str {
            &self.name
        }

        async fn sync(&self, _db: &DaemonDb) -> Result<SyncResult> {
            Ok(SyncResult {
                records_synced: self.records,
                errors: vec![],
            })
        }
    }

    #[test]
    fn registry_starts_empty() {
        let registry = DataSourceRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn registry_register_and_iterate() {
        let mut registry = DataSourceRegistry::new();
        registry.register(Arc::new(FakeSource {
            name: "fake-1".to_string(),
            records: 10,
        }));
        registry.register(Arc::new(FakeSource {
            name: "fake-2".to_string(),
            records: 5,
        }));

        assert_eq!(registry.len(), 2);
        let names: Vec<&str> = registry.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["fake-1", "fake-2"]);
    }
}
