//! Metrics Collector pipeline — Phase 3: Observation Layer.
//!
//! Periodically collects performance metrics for published content:
//! 1. Counts published content per pipeline
//! 2. Sums LLM costs from content records
//! 3. Estimates revenue based on configurable CPC rates
//! 4. Syncs real data from external sources (WordPress Stats, etc.)
//! 5. Records everything in daemon_metrics and daemon_revenue tables
//! 6. Updates active goals with current measured values

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{
    ContentFilter, ContentOutput, ContentStatus, CreateRevenue, GoalFilter, GoalMetricType,
    GoalStatus, LogLevel, MetricType, RevenueSource,
};
use serde::Deserialize;

use crate::data_sources::google_adsense::{AdSenseClient, AdSenseConfig};
use crate::data_sources::google_search_console::{SearchConsoleClient, SearchConsoleConfig};
use crate::data_sources::wordpress_stats::{WordPressStatsClient, WordPressStatsConfig};
use crate::pipeline::{Pipeline, PipelineContext};

/// Metrics Collector pipeline implementation.
pub struct MetricsCollectorPipeline;

/// Configuration for the metrics collector pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsCollectorConfig {
    /// CPC estimation settings.
    #[serde(default)]
    pub cpc_estimation: CpcEstimation,
    /// Number of days to look back for metrics.
    #[serde(default = "default_lookback_days")]
    pub lookback_days: i64,
    /// Optional WordPress Stats configuration for real pageview data.
    #[serde(default)]
    pub wordpress_stats: Option<WordPressStatsConfig>,
    /// Optional Google Search Console configuration for SEO metrics.
    #[serde(default)]
    pub search_console: Option<SearchConsoleConfig>,
    /// Optional Google AdSense configuration for real revenue data.
    #[serde(default)]
    pub adsense: Option<AdSenseConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CpcEstimation {
    /// Default cost-per-click for revenue estimation (USD).
    #[serde(default = "default_cpc")]
    pub default_cpc: f64,
    /// CPC overrides by niche.
    #[serde(default)]
    pub by_niche: std::collections::HashMap<String, f64>,
}

impl Default for CpcEstimation {
    fn default() -> Self {
        Self {
            default_cpc: default_cpc(),
            by_niche: std::collections::HashMap::new(),
        }
    }
}

impl Default for MetricsCollectorConfig {
    fn default() -> Self {
        Self {
            cpc_estimation: CpcEstimation::default(),
            lookback_days: default_lookback_days(),
            wordpress_stats: None,
            search_console: None,
            adsense: None,
        }
    }
}

fn default_cpc() -> f64 {
    0.05
}

fn default_lookback_days() -> i64 {
    7
}

#[async_trait]
impl Pipeline for MetricsCollectorPipeline {
    fn strategy(&self) -> &str {
        "metrics_collector"
    }

    fn display_name(&self) -> &str {
        "Metrics Collector"
    }

    async fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _parsed: MetricsCollectorConfig = serde_json::from_value(config.clone())?;
        Ok(())
    }

    async fn execute(&self, ctx: &PipelineContext) -> Result<Vec<ContentOutput>> {
        let config: MetricsCollectorConfig = ctx.pipeline.config()?;
        let db = &ctx.db;
        let now = chrono::Utc::now().timestamp();
        let lookback_start = now - config.lookback_days * 86400;

        ctx.log_info(&format!(
            "Starting metrics collection (lookback: {}d)",
            config.lookback_days
        ))
        .await;

        // 1. Count published content across all pipelines.
        let all_pipelines = db.list_pipelines(false).await?;
        let mut total_published = 0i64;
        let mut total_llm_cost = 0.0f64;

        for pipe in &all_pipelines {
            let content_filter = ContentFilter {
                pipeline_id: Some(pipe.id.clone()),
                status: Some(ContentStatus::Published),
                since_days: Some(config.lookback_days),
                ..Default::default()
            };
            let content = db.list_content(&content_filter).await?;
            let count = content.len() as i64;
            total_published += count;

            // Sum LLM costs for this pipeline.
            let pipe_cost: f64 = content.iter().filter_map(|c| c.llm_cost_usd).sum();
            total_llm_cost += pipe_cost;

            // Record content count metric.
            if count > 0 {
                db.insert_metric(
                    &pipe.id,
                    None,
                    MetricType::Views, // Using views as "content count" for now
                    count as f64,
                    "metrics_collector",
                    lookback_start,
                    now,
                )
                .await?;
            }

            // 2. Estimate revenue based on CPC.
            // For now, use a simple formula: estimated_clicks = content_count * 10 (conservative)
            // estimated_revenue = estimated_clicks * CPC
            if count > 0 {
                let cpc = config.cpc_estimation.default_cpc;
                let estimated_clicks = count * 10; // Conservative: 10 clicks per article
                let estimated_revenue = estimated_clicks as f64 * cpc;

                // Record clicks metric.
                db.insert_metric(
                    &pipe.id,
                    None,
                    MetricType::Clicks,
                    estimated_clicks as f64,
                    "metrics_collector_estimate",
                    lookback_start,
                    now,
                )
                .await?;

                // Record estimated revenue.
                if estimated_revenue > 0.0 {
                    let rev_input = CreateRevenue {
                        content_id: None,
                        pipeline_id: pipe.id.clone(),
                        source: RevenueSource::Estimated,
                        amount: estimated_revenue,
                        currency: None,
                        period_start: lookback_start,
                        period_end: now,
                        external_id: None,
                        metadata: Some(serde_json::json!({
                            "content_count": count,
                            "estimated_clicks": estimated_clicks,
                            "cpc": cpc,
                            "lookback_days": config.lookback_days,
                        })),
                    };
                    db.create_revenue(&rev_input).await?;

                    ctx.log(
                        LogLevel::Info,
                        &format!(
                            "Pipeline '{}': {} articles, ~{} clicks, ~${:.2} estimated revenue",
                            pipe.id, count, estimated_clicks, estimated_revenue
                        ),
                    )
                    .await;
                }
            }
        }

        let total_estimated_revenue =
            total_published as f64 * 10.0 * config.cpc_estimation.default_cpc;

        ctx.log_info(&format!(
            "Metrics collection complete: {} published items, ${:.4} LLM cost, ${:.2} estimated revenue",
            total_published, total_llm_cost, total_estimated_revenue
        ))
        .await;

        // 3. Sync real data from external sources (WordPress Stats, etc.).
        if let Some(wp_config) = &config.wordpress_stats {
            match WordPressStatsClient::new(wp_config.clone()) {
                Ok(wp_client) => {
                    use crate::data_sources::DataSource;
                    ctx.log_info("Syncing WordPress Stats...").await;
                    match wp_client.sync(db).await {
                        Ok(sync_result) => {
                            ctx.log_info(&format!(
                                "WordPress Stats: {} records synced, {} errors",
                                sync_result.records_synced,
                                sync_result.errors.len()
                            ))
                            .await;
                            for err in &sync_result.errors {
                                ctx.log(LogLevel::Warn, &format!("WP Stats: {err}")).await;
                            }
                        }
                        Err(e) => {
                            ctx.log(
                                LogLevel::Warn,
                                &format!("WordPress Stats sync failed: {e:#}"),
                            )
                            .await;
                        }
                    }
                }
                Err(e) => {
                    ctx.log(
                        LogLevel::Warn,
                        &format!("Failed to create WordPress Stats client: {e:#}"),
                    )
                    .await;
                }
            }
        }

        // 3b. Sync real data from Google Search Console.
        if let Some(sc_config) = &config.search_console {
            match SearchConsoleClient::new(sc_config.clone()) {
                Ok(sc_client) => {
                    use crate::data_sources::DataSource as _;
                    ctx.log_info("Syncing Google Search Console...").await;
                    match sc_client.sync(db).await {
                        Ok(sync_result) => {
                            ctx.log_info(&format!(
                                "Search Console: {} records synced, {} errors",
                                sync_result.records_synced,
                                sync_result.errors.len()
                            ))
                            .await;
                            for err in &sync_result.errors {
                                ctx.log(LogLevel::Warn, &format!("SC: {err}")).await;
                            }
                        }
                        Err(e) => {
                            ctx.log(
                                LogLevel::Warn,
                                &format!("Search Console sync failed: {e:#}"),
                            )
                            .await;
                        }
                    }
                }
                Err(e) => {
                    ctx.log(
                        LogLevel::Warn,
                        &format!("Failed to create Search Console client: {e:#}"),
                    )
                    .await;
                }
            }
        }

        // 3c. Sync real revenue data from Google AdSense.
        if let Some(ads_config) = &config.adsense {
            match AdSenseClient::new(ads_config.clone()) {
                Ok(ads_client) => {
                    use crate::data_sources::DataSource as _;
                    ctx.log_info("Syncing Google AdSense...").await;
                    match ads_client.sync(db).await {
                        Ok(sync_result) => {
                            ctx.log_info(&format!(
                                "AdSense: {} records synced, {} errors",
                                sync_result.records_synced,
                                sync_result.errors.len()
                            ))
                            .await;
                            for err in &sync_result.errors {
                                ctx.log(LogLevel::Warn, &format!("AdSense: {err}")).await;
                            }
                        }
                        Err(e) => {
                            ctx.log(LogLevel::Warn, &format!("AdSense sync failed: {e:#}"))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    ctx.log(
                        LogLevel::Warn,
                        &format!("Failed to create AdSense client: {e:#}"),
                    )
                    .await;
                }
            }
        }

        // 4. Update active goals with current measured values.
        let goals = db
            .list_goals(&GoalFilter {
                status: Some(GoalStatus::Active),
                ..Default::default()
            })
            .await?;
        for goal in &goals {
            let metric: GoalMetricType = match goal.metric_type.parse() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let current = match metric {
                GoalMetricType::Revenue => {
                    let summary = db.revenue_summary(config.lookback_days).await?;
                    summary.total_usd
                }
                GoalMetricType::ContentCount => total_published as f64,
                GoalMetricType::CostLimit => total_llm_cost,
                GoalMetricType::Pageviews => {
                    let since = now - config.lookback_days * 86400;
                    db.sum_metrics(MetricType::Views, since, None).await?
                }
                GoalMetricType::Clicks => {
                    let since = now - config.lookback_days * 86400;
                    db.sum_metrics(MetricType::Clicks, since, None).await?
                }
                // Other metrics (CTR, Subscribers, Custom) require more specialized integration.
                _ => continue,
            };
            db.update_goal_current_value(&goal.id, current).await?;

            // Auto-achieve goals that hit 100%.
            if current >= goal.target_value {
                db.set_goal_status(&goal.id, GoalStatus::Achieved).await?;
                ctx.log_info(&format!(
                    "Goal achieved: {} ({:.2}/{:.2} {})",
                    goal.name, current, goal.target_value, goal.target_unit
                ))
                .await;
            }
        }

        // The metrics collector doesn't produce "content" in the traditional sense,
        // so we return an empty Vec. The real output is in the daemon_metrics and
        // daemon_revenue tables.
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_config_defaults() {
        let config: MetricsCollectorConfig =
            serde_json::from_value(serde_json::json!({})).expect("parse empty config");
        assert_eq!(config.lookback_days, 7);
        assert!((config.cpc_estimation.default_cpc - 0.05).abs() < 0.001);
    }

    #[test]
    fn parse_config_custom() {
        let config: MetricsCollectorConfig = serde_json::from_value(serde_json::json!({
            "lookback_days": 30,
            "cpc_estimation": {
                "default_cpc": 0.15,
                "by_niche": {
                    "concursos": 0.25,
                    "tech": 0.10
                }
            }
        }))
        .expect("parse custom config");
        assert_eq!(config.lookback_days, 30);
        assert!((config.cpc_estimation.default_cpc - 0.15).abs() < 0.001);
        assert_eq!(config.cpc_estimation.by_niche.len(), 2);
    }
}
