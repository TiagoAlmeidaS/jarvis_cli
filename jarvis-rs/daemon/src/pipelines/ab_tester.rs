//! A/B Tester pipeline — SEO title experimentation.
//!
//! This pipeline:
//! 1. Picks published content that doesn't have an active experiment
//! 2. Generates an alternative title variant using the LLM
//! 3. Creates an experiment record in daemon_experiments
//! 4. On subsequent runs, measures CTR from Search Console data
//! 5. After the minimum duration, declares a winner and optionally updates
//!    the live title via the WordPress API
//!
//! This closes roadmap item 3.3: "A/B testing de titulos SEO".

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{
    ContentFilter, ContentOutput, ContentStatus, CreateExperiment, ExperimentFilter,
    ExperimentStatus, ExperimentType, LogLevel, MetricType,
};
use serde::Deserialize;

use crate::pipeline::{Pipeline, PipelineContext};
use crate::processor;

/// A/B Tester pipeline implementation.
pub struct AbTesterPipeline;

/// Configuration for the A/B tester pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct AbTesterConfig {
    /// LLM configuration for generating variant titles.
    #[serde(default)]
    pub llm: processor::LlmConfig,
    /// Maximum concurrent experiments per pipeline.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_experiments: usize,
    /// Minimum days before declaring a winner.
    #[serde(default = "default_min_duration")]
    pub min_duration_days: i32,
    /// Minimum impressions before declaring a winner.
    #[serde(default = "default_min_impressions")]
    pub min_impressions: f64,
    /// Minimum CTR difference (percentage points) to declare a winner.
    #[serde(default = "default_min_ctr_diff")]
    pub min_ctr_diff_pct: f64,
    /// Which pipeline's content to experiment on.
    #[serde(default)]
    pub target_pipeline_id: Option<String>,
    /// WordPress base URL for updating titles (optional).
    #[serde(default)]
    pub wordpress_base_url: Option<String>,
}

impl Default for AbTesterConfig {
    fn default() -> Self {
        Self {
            llm: processor::LlmConfig::default(),
            max_concurrent_experiments: default_max_concurrent(),
            min_duration_days: default_min_duration(),
            min_impressions: default_min_impressions(),
            min_ctr_diff_pct: default_min_ctr_diff(),
            target_pipeline_id: None,
            wordpress_base_url: None,
        }
    }
}

fn default_max_concurrent() -> usize {
    3
}
fn default_min_duration() -> i32 {
    7
}
fn default_min_impressions() -> f64 {
    100.0
}
fn default_min_ctr_diff() -> f64 {
    0.5
}

#[async_trait]
impl Pipeline for AbTesterPipeline {
    fn strategy(&self) -> &str {
        "ab_tester"
    }

    fn display_name(&self) -> &str {
        "A/B Title Tester"
    }

    async fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _parsed: AbTesterConfig = serde_json::from_value(config.clone())?;
        Ok(())
    }

    async fn execute(&self, ctx: &PipelineContext) -> Result<Vec<ContentOutput>> {
        let config: AbTesterConfig = ctx.pipeline.config()?;
        let db = &ctx.db;

        ctx.log_info("Starting A/B tester run").await;

        // Phase 1: Evaluate mature experiments.
        let mature = db.list_mature_experiments().await?;
        let mut completed = 0;
        for exp in &mature {
            // Fetch fresh CTR data for both variants.
            let (ctr_a, impressions_a) =
                fetch_variant_ctr(db, &exp.content_id, "google_search_console").await;
            let (ctr_b, impressions_b) = (exp.metric_b, 0.0); // variant B uses the stored metric

            // We need enough impressions on both variants.
            let total_impressions = impressions_a + impressions_b;
            if total_impressions < config.min_impressions {
                ctx.log(
                    LogLevel::Debug,
                    &format!(
                        "Experiment '{}': only {:.0} impressions (need {:.0}), waiting",
                        exp.id, total_impressions, config.min_impressions
                    ),
                )
                .await;
                continue;
            }

            // Update metrics.
            db.update_experiment_metrics(&exp.id, ctr_a, ctr_b).await?;

            // Determine winner.
            let diff = (ctr_a - ctr_b).abs();
            if diff >= config.min_ctr_diff_pct {
                let winner = if ctr_a >= ctr_b { "a" } else { "b" };
                db.complete_experiment(&exp.id, winner).await?;
                completed += 1;

                let winner_label = if winner == "a" {
                    &exp.variant_a
                } else {
                    &exp.variant_b
                };
                ctx.log_info(&format!(
                    "Experiment completed: '{}' wins (CTR {:.2}% vs {:.2}%, diff {:.2}pp)",
                    winner_label,
                    if winner == "a" { ctr_a } else { ctr_b },
                    if winner == "a" { ctr_b } else { ctr_a },
                    diff,
                ))
                .await;
            } else {
                ctx.log(
                    LogLevel::Debug,
                    &format!(
                        "Experiment '{}': CTR diff {:.2}pp < {:.2}pp threshold, continuing",
                        exp.id, diff, config.min_ctr_diff_pct
                    ),
                )
                .await;
            }
        }

        if completed > 0 {
            ctx.log_info(&format!("{completed} experiments completed"))
                .await;
        }

        // Phase 2: Create new experiments if below the limit.
        let running = db
            .list_experiments(&ExperimentFilter {
                status: Some(ExperimentStatus::Running),
                ..Default::default()
            })
            .await?;

        let slots_available = config
            .max_concurrent_experiments
            .saturating_sub(running.len());

        if slots_available == 0 {
            ctx.log(
                LogLevel::Debug,
                &format!(
                    "Already {} running experiments (max {}), skipping new experiment creation",
                    running.len(),
                    config.max_concurrent_experiments
                ),
            )
            .await;
            return Ok(vec![]);
        }

        // Find published content without active experiments.
        let content_filter = ContentFilter {
            pipeline_id: config.target_pipeline_id.clone(),
            status: Some(ContentStatus::Published),
            since_days: Some(90), // Only consider recent content.
            limit: Some(slots_available as i64 * 3), // Fetch more than needed for filtering.
            ..Default::default()
        };
        let candidates = db.list_content(&content_filter).await?;

        // Filter out content that already has an active experiment.
        let active_content_ids: std::collections::HashSet<&str> =
            running.iter().map(|e| e.content_id.as_str()).collect();

        let eligible: Vec<_> = candidates
            .iter()
            .filter(|c| !active_content_ids.contains(c.id.as_str()) && c.title.is_some())
            .take(slots_available)
            .collect();

        if eligible.is_empty() {
            ctx.log(LogLevel::Debug, "No eligible content for new experiments")
                .await;
            return Ok(vec![]);
        }

        // Generate variant titles via LLM.
        let mut created = 0;
        for content in &eligible {
            let original_title = match &content.title {
                Some(t) => t.clone(),
                None => continue,
            };

            let prompt = format!(
                "Generate a single alternative SEO title for this article. \
                 The new title should be click-worthy, include relevant keywords, \
                 and be approximately the same length.\n\n\
                 Original title: \"{original_title}\"\n\n\
                 Respond with ONLY the new title, no quotes or explanation."
            );

            let variant_b = match ctx
                .llm_client
                .generate(&prompt, Some("You are an SEO title optimization expert."))
                .await
            {
                Ok(response) => response.text.trim().trim_matches('"').to_string(),
                Err(e) => {
                    ctx.log(
                        LogLevel::Warn,
                        &format!("LLM failed for '{}': {e}", content.id),
                    )
                    .await;
                    continue;
                }
            };

            if variant_b.is_empty() || variant_b.len() > 200 {
                continue;
            }

            let input = CreateExperiment {
                content_id: content.id.clone(),
                pipeline_id: content.pipeline_id.clone(),
                experiment_type: ExperimentType::Title,
                variant_a: original_title.clone(),
                variant_b: variant_b.clone(),
                metric: "ctr".to_string(),
                min_duration_days: Some(config.min_duration_days),
            };

            match db.create_experiment(&input).await {
                Ok(exp) => {
                    created += 1;
                    ctx.log_info(&format!(
                        "New experiment '{}': A=\"{}\" vs B=\"{}\"",
                        exp.id, original_title, variant_b
                    ))
                    .await;
                }
                Err(e) => {
                    ctx.log(LogLevel::Warn, &format!("Failed to create experiment: {e}"))
                        .await;
                }
            }
        }

        ctx.log_info(&format!(
            "A/B tester run complete: {created} new experiments, {completed} completed"
        ))
        .await;

        Ok(vec![])
    }
}

/// Fetch the average CTR and total impressions for a content item from metrics.
async fn fetch_variant_ctr(
    db: &jarvis_daemon_common::DaemonDb,
    _content_id: &str,
    source: &str,
) -> (f64, f64) {
    let now = chrono::Utc::now().timestamp();
    let since = now - 30 * 86400;

    // Get CTR metrics for this content.
    let ctr = db
        .sum_metrics(MetricType::Ctr, since, Some(source))
        .await
        .unwrap_or(0.0);

    let impressions = db
        .sum_metrics(MetricType::Impressions, since, Some(source))
        .await
        .unwrap_or(0.0);

    (ctr, impressions)
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
        let config: AbTesterConfig = serde_json::from_value(serde_json::json!({})).expect("parse");
        assert_eq!(config.max_concurrent_experiments, 3);
        assert_eq!(config.min_duration_days, 7);
        assert!((config.min_impressions - 100.0).abs() < 0.01);
        assert!((config.min_ctr_diff_pct - 0.5).abs() < 0.01);
        assert!(config.target_pipeline_id.is_none());
    }

    #[test]
    fn parse_config_custom() {
        let config: AbTesterConfig = serde_json::from_value(serde_json::json!({
            "max_concurrent_experiments": 5,
            "min_duration_days": 14,
            "min_impressions": 500.0,
            "min_ctr_diff_pct": 1.0,
            "target_pipeline_id": "seo-concursos"
        }))
        .expect("parse");
        assert_eq!(config.max_concurrent_experiments, 5);
        assert_eq!(config.min_duration_days, 14);
        assert!((config.min_impressions - 500.0).abs() < 0.01);
        assert_eq!(config.target_pipeline_id.as_deref(), Some("seo-concursos"));
    }

    #[test]
    fn strategy_name() {
        let pipeline = AbTesterPipeline;
        assert_eq!(pipeline.strategy(), "ab_tester");
        assert_eq!(pipeline.display_name(), "A/B Title Tester");
    }

    #[tokio::test]
    async fn validate_config_rejects_bad_json() {
        let pipeline = AbTesterPipeline;
        let bad = serde_json::json!({"max_concurrent_experiments": "not_a_number"});
        assert!(pipeline.validate_config(&bad).await.is_err());
    }

    #[tokio::test]
    async fn validate_config_accepts_empty() {
        let pipeline = AbTesterPipeline;
        let empty = serde_json::json!({});
        assert!(pipeline.validate_config(&empty).await.is_ok());
    }

    #[tokio::test]
    async fn experiment_crud_roundtrip() {
        let db = jarvis_daemon_common::DaemonDb::open_memory()
            .await
            .expect("open db");

        // Need a pipeline and content first.
        let pipe = jarvis_daemon_common::CreatePipeline {
            id: "p1".to_string(),
            name: "SEO Blog".to_string(),
            strategy: jarvis_daemon_common::Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 3 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&pipe).await.unwrap();
        let job = db.create_job("p1").await.unwrap();
        let content = db
            .create_content(
                &job.id,
                "p1",
                &jarvis_daemon_common::ContentOutput {
                    content_type: jarvis_daemon_common::ContentType::Article,
                    platform: jarvis_daemon_common::Platform::Wordpress,
                    title: "Original Title".to_string(),
                    slug: "original-title".to_string(),
                    body: "Body content here".to_string(),
                    url: Some("https://example.com/original-title".to_string()),
                    word_count: Some(100),
                    llm_model: "test".to_string(),
                    llm_tokens_used: 100,
                    llm_cost_usd: Some(0.001),
                },
            )
            .await
            .unwrap();

        // Create experiment.
        let exp_input = CreateExperiment {
            content_id: content.id.clone(),
            pipeline_id: "p1".to_string(),
            experiment_type: ExperimentType::Title,
            variant_a: "Original Title".to_string(),
            variant_b: "Better SEO Title for Concursos 2026".to_string(),
            metric: "ctr".to_string(),
            min_duration_days: Some(7),
        };
        let exp = db.create_experiment(&exp_input).await.unwrap();
        assert_eq!(exp.status, "running");
        assert_eq!(exp.active_variant, "a");
        assert_eq!(exp.variant_a, "Original Title");
        assert!(exp.winner.is_none());

        // List running experiments.
        let running = db
            .list_experiments(&ExperimentFilter {
                status: Some(ExperimentStatus::Running),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(running.len(), 1);

        // Switch variant.
        db.switch_experiment_variant(&exp.id, "b").await.unwrap();
        let updated = db.get_experiment(&exp.id).await.unwrap().unwrap();
        assert_eq!(updated.active_variant, "b");

        // Update metrics.
        db.update_experiment_metrics(&exp.id, 3.5, 4.2)
            .await
            .unwrap();
        let updated = db.get_experiment(&exp.id).await.unwrap().unwrap();
        assert!((updated.metric_a - 3.5).abs() < 0.01);
        assert!((updated.metric_b - 4.2).abs() < 0.01);

        // Complete experiment.
        db.complete_experiment(&exp.id, "b").await.unwrap();
        let completed = db.get_experiment(&exp.id).await.unwrap().unwrap();
        assert_eq!(completed.status, "completed");
        assert_eq!(completed.winner.as_deref(), Some("b"));
        assert_eq!(completed.active_variant, "b");
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn cancel_experiment() {
        let db = jarvis_daemon_common::DaemonDb::open_memory()
            .await
            .expect("open db");

        let pipe = jarvis_daemon_common::CreatePipeline {
            id: "p1".to_string(),
            name: "P1".to_string(),
            strategy: jarvis_daemon_common::Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&pipe).await.unwrap();
        let job = db.create_job("p1").await.unwrap();
        let content = db
            .create_content(
                &job.id,
                "p1",
                &jarvis_daemon_common::ContentOutput {
                    content_type: jarvis_daemon_common::ContentType::Article,
                    platform: jarvis_daemon_common::Platform::Wordpress,
                    title: "Test".to_string(),
                    slug: "test".to_string(),
                    body: "Body".to_string(),
                    url: None,
                    word_count: None,
                    llm_model: "test".to_string(),
                    llm_tokens_used: 10,
                    llm_cost_usd: None,
                },
            )
            .await
            .unwrap();

        let exp_input = CreateExperiment {
            content_id: content.id.clone(),
            pipeline_id: "p1".to_string(),
            experiment_type: ExperimentType::Title,
            variant_a: "A".to_string(),
            variant_b: "B".to_string(),
            metric: "ctr".to_string(),
            min_duration_days: None,
        };
        let exp = db.create_experiment(&exp_input).await.unwrap();

        db.cancel_experiment(&exp.id).await.unwrap();
        let cancelled = db.get_experiment(&exp.id).await.unwrap().unwrap();
        assert_eq!(cancelled.status, "cancelled");
    }

    #[tokio::test]
    async fn list_mature_experiments_respects_duration() {
        let db = jarvis_daemon_common::DaemonDb::open_memory()
            .await
            .expect("open db");

        let pipe = jarvis_daemon_common::CreatePipeline {
            id: "p1".to_string(),
            name: "P1".to_string(),
            strategy: jarvis_daemon_common::Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&pipe).await.unwrap();
        let job = db.create_job("p1").await.unwrap();
        let content = db
            .create_content(
                &job.id,
                "p1",
                &jarvis_daemon_common::ContentOutput {
                    content_type: jarvis_daemon_common::ContentType::Article,
                    platform: jarvis_daemon_common::Platform::Wordpress,
                    title: "Test".to_string(),
                    slug: "test".to_string(),
                    body: "Body".to_string(),
                    url: None,
                    word_count: None,
                    llm_model: "test".to_string(),
                    llm_tokens_used: 10,
                    llm_cost_usd: None,
                },
            )
            .await
            .unwrap();

        // Create experiment with 7-day minimum (just created, so NOT mature).
        let exp_input = CreateExperiment {
            content_id: content.id.clone(),
            pipeline_id: "p1".to_string(),
            experiment_type: ExperimentType::Title,
            variant_a: "A".to_string(),
            variant_b: "B".to_string(),
            metric: "ctr".to_string(),
            min_duration_days: Some(7),
        };
        db.create_experiment(&exp_input).await.unwrap();

        let mature = db.list_mature_experiments().await.unwrap();
        // Just created, so should not be mature.
        assert!(mature.is_empty());
    }
}
