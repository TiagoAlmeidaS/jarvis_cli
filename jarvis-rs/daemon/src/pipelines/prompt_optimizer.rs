//! Prompt Optimizer pipeline — Phase 3.5: Auto-optimization of prompts.
//!
//! Periodically analyzes which prompt parameters produce the best-performing content:
//! 1. Scans content that has metrics but no prompt score record yet
//! 2. Extracts prompt parameters from the originating pipeline config
//! 3. Correlates with CTR, clicks, and revenue metrics
//! 4. Builds aggregated performance summaries per prompt hash
//! 5. Uses LLM to suggest parameter optimizations
//! 6. Creates proposals to update pipeline configs

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{
    ActionType, ContentFilter, ContentOutput, CreatePromptScore, CreateProposal, LogLevel,
    PromptOptimizationSuggestion, RiskLevel,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::pipeline::{Pipeline, PipelineContext};
use crate::processor;

/// Prompt Optimizer pipeline implementation.
pub struct PromptOptimizerPipeline;

/// Configuration for the prompt optimizer pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptOptimizerConfig {
    /// LLM configuration.
    #[serde(default)]
    pub llm: processor::LlmConfig,
    /// Minimum number of content items with metrics before analyzing.
    #[serde(default = "default_min_content_for_analysis")]
    pub min_content_for_analysis: usize,
    /// Target pipeline IDs to optimize (empty = all seo_blog pipelines).
    #[serde(default)]
    pub target_pipeline_ids: Vec<String>,
    /// Days to look back for content.
    #[serde(default = "default_lookback_days")]
    pub lookback_days: i64,
    /// Weight for CTR in composite score.
    #[serde(default = "default_ctr_weight")]
    pub ctr_weight: f64,
    /// Weight for clicks in composite score.
    #[serde(default = "default_clicks_weight")]
    pub clicks_weight: f64,
    /// Weight for revenue in composite score.
    #[serde(default = "default_revenue_weight")]
    pub revenue_weight: f64,
}

impl Default for PromptOptimizerConfig {
    fn default() -> Self {
        Self {
            llm: processor::LlmConfig::default(),
            min_content_for_analysis: default_min_content_for_analysis(),
            target_pipeline_ids: vec![],
            lookback_days: default_lookback_days(),
            ctr_weight: default_ctr_weight(),
            clicks_weight: default_clicks_weight(),
            revenue_weight: default_revenue_weight(),
        }
    }
}

fn default_min_content_for_analysis() -> usize {
    5
}
fn default_lookback_days() -> i64 {
    30
}
fn default_ctr_weight() -> f64 {
    0.4
}
fn default_clicks_weight() -> f64 {
    0.3
}
fn default_revenue_weight() -> f64 {
    0.3
}

#[async_trait]
impl Pipeline for PromptOptimizerPipeline {
    fn strategy(&self) -> &str {
        "prompt_optimizer"
    }

    fn display_name(&self) -> &str {
        "Prompt Optimizer"
    }

    async fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _parsed: PromptOptimizerConfig = serde_json::from_value(config.clone())?;
        Ok(())
    }

    async fn execute(&self, ctx: &PipelineContext) -> Result<Vec<ContentOutput>> {
        let config: PromptOptimizerConfig = ctx.pipeline.config()?;
        let db = &ctx.db;

        ctx.log_info("Starting prompt optimization analysis").await;

        // 1. Find target pipelines (seo_blog pipelines by default).
        let all_pipelines = db.list_pipelines(false).await?;
        let target_pipelines: Vec<_> = all_pipelines
            .iter()
            .filter(|p| {
                if !config.target_pipeline_ids.is_empty() {
                    config.target_pipeline_ids.contains(&p.id)
                } else {
                    p.strategy == "seo_blog"
                }
            })
            .collect();

        if target_pipelines.is_empty() {
            ctx.log_info("No target pipelines found for optimization")
                .await;
            return Ok(vec![]);
        }

        let mut total_scored = 0;
        let mut total_suggestions = 0;

        for pipeline in &target_pipelines {
            ctx.log_info(&format!(
                "Analyzing pipeline '{}' ({})",
                pipeline.name, pipeline.id
            ))
            .await;

            // 2. Get content for this pipeline.
            let content = db
                .list_content(&ContentFilter {
                    pipeline_id: Some(pipeline.id.clone()),
                    since_days: Some(config.lookback_days),
                    ..Default::default()
                })
                .await?;

            let published: Vec<_> = content.iter().filter(|c| c.status == "published").collect();

            if published.len() < config.min_content_for_analysis {
                ctx.log(
                    LogLevel::Debug,
                    &format!(
                        "Pipeline '{}': only {} published items (need {}), skipping",
                        pipeline.id,
                        published.len(),
                        config.min_content_for_analysis,
                    ),
                )
                .await;
                continue;
            }

            // 3. Extract prompt parameters from pipeline config.
            let pipeline_config: serde_json::Value =
                serde_json::from_str(&pipeline.config_json).unwrap_or_default();
            let prompt_params = extract_prompt_params(&pipeline_config);
            let prompt_hash = hash_params(&prompt_params);

            // 4. Score content that doesn't have a prompt_score yet.
            for content_item in &published {
                let existing = db.get_prompt_score_by_content(&content_item.id).await?;
                if existing.is_some() {
                    continue;
                }

                // Get metrics for this content.
                let ctr = get_content_metric(db, &content_item.id, "ctr").await;
                let clicks = get_content_metric(db, &content_item.id, "clicks").await as i64;
                let impressions =
                    get_content_metric(db, &content_item.id, "impressions").await as i64;
                let revenue = get_content_metric(db, &content_item.id, "revenue").await;

                // Only score if there are any metrics.
                if clicks == 0 && impressions == 0 && revenue == 0.0 {
                    continue;
                }

                let score = db
                    .create_prompt_score(&CreatePromptScore {
                        pipeline_id: pipeline.id.clone(),
                        content_id: content_item.id.clone(),
                        prompt_hash: prompt_hash.clone(),
                        params_json: prompt_params.clone(),
                    })
                    .await?;

                db.update_prompt_score_metrics(&score.id, ctr, clicks, impressions, revenue)
                    .await?;

                total_scored += 1;
            }

            // 5. Get performance summary.
            let summaries = db.prompt_performance_summary(&pipeline.id).await?;

            if summaries.is_empty() {
                continue;
            }

            ctx.log_info(&format!(
                "Pipeline '{}': {} prompt variants scored, best composite: {:.2}",
                pipeline.id,
                summaries.len(),
                summaries.first().map(|s| s.composite_score).unwrap_or(0.0),
            ))
            .await;

            // 6. Use LLM to suggest optimizations.
            let suggestions = generate_suggestions(ctx, &config, pipeline, &summaries).await?;

            // 7. Create proposals for significant suggestions.
            for suggestion in &suggestions {
                if suggestion.expected_improvement_pct < 5.0 {
                    continue;
                }

                let proposal = CreateProposal {
                    pipeline_id: Some(pipeline.id.clone()),
                    action_type: ActionType::ModifyPipeline,
                    title: format!(
                        "Optimize prompt: {} -> {}",
                        suggestion.parameter, suggestion.suggested_value
                    ),
                    description: format!(
                        "Change '{}' from '{}' to '{}'. {}",
                        suggestion.parameter,
                        suggestion.current_value,
                        suggestion.suggested_value,
                        suggestion.reason,
                    ),
                    reasoning: format!(
                        "Expected improvement: {:.1}%. Based on analysis of {} content items.",
                        suggestion.expected_improvement_pct,
                        summaries.iter().map(|s| s.content_count).sum::<i64>(),
                    ),
                    confidence: (suggestion.expected_improvement_pct / 100.0).clamp(0.3, 0.85),
                    risk_level: RiskLevel::Low,
                    proposed_config: Some(build_config_patch(
                        &pipeline_config,
                        &suggestion.parameter,
                        &suggestion.suggested_value,
                    )),
                    metrics_snapshot: None,
                    auto_approvable: false, // Always require human review for prompt changes.
                    expires_in_hours: Some(336), // 14 days
                };

                if let Ok(p) = db.create_proposal(&proposal).await {
                    total_suggestions += 1;
                    ctx.log_info(&format!(
                        "Optimization proposal: {} (expected +{:.1}%)",
                        p.title, suggestion.expected_improvement_pct,
                    ))
                    .await;
                }
            }
        }

        ctx.log_info(&format!(
            "Prompt optimization complete: {total_scored} items scored, {total_suggestions} suggestions created",
        ))
        .await;

        Ok(vec![])
    }
}

/// Extract SEO-relevant prompt parameters from a pipeline config.
fn extract_prompt_params(config: &serde_json::Value) -> serde_json::Value {
    let seo = config.get("seo").unwrap_or(config);
    serde_json::json!({
        "niche": seo.get("niche").and_then(|v| v.as_str()).unwrap_or("Tecnologia"),
        "target_audience": seo.get("target_audience").and_then(|v| v.as_str()).unwrap_or("desenvolvedores e entusiastas"),
        "language": seo.get("language").and_then(|v| v.as_str()).unwrap_or("pt-BR"),
        "tone": seo.get("tone").and_then(|v| v.as_str()).unwrap_or("informativo e envolvente"),
        "min_word_count": seo.get("min_word_count").and_then(|v| v.as_u64()).unwrap_or(800),
        "max_word_count": seo.get("max_word_count").and_then(|v| v.as_u64()).unwrap_or(2000),
    })
}

/// Create a stable hash of prompt parameters.
fn hash_params(params: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(params).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(&hasher.finalize()[..8]) // Short 16-char hash
}

/// Get a specific metric value for a content item.
async fn get_content_metric(
    db: &jarvis_daemon_common::DaemonDb,
    content_id: &str,
    metric_type: &str,
) -> f64 {
    db.sum_content_metric(content_id, metric_type)
        .await
        .unwrap_or(0.0)
}

/// Use LLM to generate optimization suggestions based on performance data.
async fn generate_suggestions(
    ctx: &PipelineContext,
    _config: &PromptOptimizerConfig,
    pipeline: &jarvis_daemon_common::DaemonPipeline,
    summaries: &[jarvis_daemon_common::PromptPerformanceSummary],
) -> Result<Vec<PromptOptimizationSuggestion>> {
    let system_prompt = build_system_prompt();
    let user_prompt = build_user_prompt(pipeline, summaries);

    let response = ctx
        .llm_client
        .generate_json(&user_prompt, Some(&system_prompt))
        .await;

    let json = match response {
        Ok(j) => j,
        Err(e) => {
            ctx.log(LogLevel::Warn, &format!("LLM prompt analysis failed: {e}"))
                .await;
            return Ok(vec![]);
        }
    };

    parse_suggestions(&json)
}

fn build_system_prompt() -> String {
    String::from(
        "You are an SEO prompt optimization expert. Analyze the performance data of content \
         generated with different prompt parameters and suggest specific improvements.\n\n\
         RULES:\n\
         - Focus on actionable, specific parameter changes\n\
         - Base suggestions on actual performance differences in the data\n\
         - Estimate improvement conservatively (5-30% range)\n\
         - Consider that CTR, clicks, and revenue are all important metrics\n\
         - Suggest changes to: niche, target_audience, tone, min_word_count, max_word_count\n\n\
         Respond in JSON format:\n\
         {\n\
           \"suggestions\": [\n\
             {\n\
               \"parameter\": \"tone\",\n\
               \"current_value\": \"informativo\",\n\
               \"suggested_value\": \"informativo com storytelling\",\n\
               \"reason\": \"Content with narrative elements shows 15% higher CTR\",\n\
               \"expected_improvement_pct\": 12.0\n\
             }\n\
           ]\n\
         }",
    )
}

fn build_user_prompt(
    pipeline: &jarvis_daemon_common::DaemonPipeline,
    summaries: &[jarvis_daemon_common::PromptPerformanceSummary],
) -> String {
    let mut prompt = format!(
        "## Pipeline: {} ({})\nCurrent config: {}\n\n## Performance by Prompt Variant\n",
        pipeline.name, pipeline.id, pipeline.config_json,
    );

    for (i, s) in summaries.iter().enumerate() {
        prompt.push_str(&format!(
            "### Variant {} (hash: {})\n\
             Parameters: {}\n\
             Content count: {}\n\
             Avg CTR: {:.2}%\n\
             Avg Clicks: {:.1}\n\
             Total Revenue: ${:.2}\n\
             Composite Score: {:.2}\n\n",
            i + 1,
            s.prompt_hash,
            serde_json::to_string_pretty(&s.params_json).unwrap_or_default(),
            s.content_count,
            s.avg_ctr,
            s.avg_clicks,
            s.total_revenue,
            s.composite_score,
        ));
    }

    prompt.push_str(
        "\nAnalyze the performance data above and suggest up to 3 specific parameter \
         changes to improve overall content performance. Focus on the biggest opportunities.",
    );
    prompt
}

/// Parse LLM suggestions from JSON response.
fn parse_suggestions(json: &serde_json::Value) -> Result<Vec<PromptOptimizationSuggestion>> {
    let suggestions = json
        .get("suggestions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut result = Vec::new();
    for s in suggestions {
        if let (Some(parameter), Some(current), Some(suggested), Some(reason)) = (
            s.get("parameter").and_then(|v| v.as_str()),
            s.get("current_value").and_then(|v| v.as_str()),
            s.get("suggested_value").and_then(|v| v.as_str()),
            s.get("reason").and_then(|v| v.as_str()),
        ) {
            result.push(PromptOptimizationSuggestion {
                parameter: parameter.to_string(),
                current_value: current.to_string(),
                suggested_value: suggested.to_string(),
                reason: reason.to_string(),
                expected_improvement_pct: s
                    .get("expected_improvement_pct")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(5.0),
            });
        }
    }
    Ok(result)
}

/// Build a JSON patch for the pipeline config with the suggested change.
fn build_config_patch(
    current_config: &serde_json::Value,
    parameter: &str,
    new_value: &str,
) -> serde_json::Value {
    let mut patch = current_config.clone();
    // Try to update in the "seo" section first, then root.
    if let Some(seo) = patch.get_mut("seo") {
        if seo.get(parameter).is_some() {
            seo[parameter] = serde_json::Value::String(new_value.to_string());
            return patch;
        }
    }
    patch[parameter] = serde_json::Value::String(new_value.to_string());
    patch
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
        let config: PromptOptimizerConfig =
            serde_json::from_value(serde_json::json!({})).expect("parse");
        assert_eq!(config.min_content_for_analysis, 5);
        assert_eq!(config.lookback_days, 30);
        assert!((config.ctr_weight - 0.4).abs() < 0.01);
        assert!((config.clicks_weight - 0.3).abs() < 0.01);
        assert!((config.revenue_weight - 0.3).abs() < 0.01);
        assert!(config.target_pipeline_ids.is_empty());
    }

    #[test]
    fn parse_config_custom() {
        let config: PromptOptimizerConfig = serde_json::from_value(serde_json::json!({
            "min_content_for_analysis": 10,
            "target_pipeline_ids": ["seo-1", "seo-2"],
            "lookback_days": 60,
            "ctr_weight": 0.5,
            "clicks_weight": 0.25,
            "revenue_weight": 0.25,
        }))
        .expect("parse");
        assert_eq!(config.min_content_for_analysis, 10);
        assert_eq!(config.target_pipeline_ids, vec!["seo-1", "seo-2"]);
        assert_eq!(config.lookback_days, 60);
    }

    #[test]
    fn strategy_name() {
        let pipeline = PromptOptimizerPipeline;
        assert_eq!(pipeline.strategy(), "prompt_optimizer");
        assert_eq!(pipeline.display_name(), "Prompt Optimizer");
    }

    #[test]
    fn extract_params_from_seo_config() {
        let config = serde_json::json!({
            "seo": {
                "niche": "Finance",
                "target_audience": "investors",
                "language": "en",
                "tone": "authoritative",
                "min_word_count": 1000,
                "max_word_count": 3000,
            }
        });
        let params = extract_prompt_params(&config);
        assert_eq!(params["niche"], "Finance");
        assert_eq!(params["target_audience"], "investors");
        assert_eq!(params["language"], "en");
        assert_eq!(params["tone"], "authoritative");
        assert_eq!(params["min_word_count"], 1000);
        assert_eq!(params["max_word_count"], 3000);
    }

    #[test]
    fn extract_params_defaults_when_missing() {
        let config = serde_json::json!({});
        let params = extract_prompt_params(&config);
        assert_eq!(params["niche"], "Tecnologia");
        assert_eq!(params["language"], "pt-BR");
    }

    #[test]
    fn hash_params_is_stable() {
        let params = serde_json::json!({"niche": "tech", "tone": "fun"});
        let h1 = hash_params(&params);
        let h2 = hash_params(&params);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16); // 8 bytes hex
    }

    #[test]
    fn hash_params_differs_for_different_values() {
        let p1 = serde_json::json!({"niche": "tech"});
        let p2 = serde_json::json!({"niche": "finance"});
        assert_ne!(hash_params(&p1), hash_params(&p2));
    }

    #[test]
    fn parse_suggestions_from_json() {
        let json = serde_json::json!({
            "suggestions": [
                {
                    "parameter": "tone",
                    "current_value": "informativo",
                    "suggested_value": "informativo com storytelling",
                    "reason": "Higher CTR with narrative style",
                    "expected_improvement_pct": 12.0
                },
                {
                    "parameter": "niche",
                    "current_value": "tech",
                    "suggested_value": "AI & Machine Learning",
                    "reason": "Trending niche with higher engagement"
                }
            ]
        });
        let suggestions = parse_suggestions(&json).expect("parse");
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].parameter, "tone");
        assert!((suggestions[0].expected_improvement_pct - 12.0).abs() < 0.01);
        // Second suggestion uses default improvement.
        assert!((suggestions[1].expected_improvement_pct - 5.0).abs() < 0.01);
    }

    #[test]
    fn parse_suggestions_empty() {
        let json = serde_json::json!({"suggestions": []});
        let suggestions = parse_suggestions(&json).expect("parse");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn build_config_patch_updates_seo_section() {
        let config = serde_json::json!({
            "seo": {"niche": "tech", "tone": "fun"},
            "publisher": {"base_url": "http://x.com"}
        });
        let patched = build_config_patch(&config, "niche", "finance");
        assert_eq!(patched["seo"]["niche"], "finance");
        assert_eq!(patched["seo"]["tone"], "fun"); // unchanged
        assert_eq!(patched["publisher"]["base_url"], "http://x.com"); // unchanged
    }

    #[test]
    fn build_config_patch_falls_back_to_root() {
        let config = serde_json::json!({"niche": "tech"});
        let patched = build_config_patch(&config, "tone", "serious");
        assert_eq!(patched["tone"], "serious");
    }

    #[tokio::test]
    async fn validate_config_accepts_empty() {
        let pipeline = PromptOptimizerPipeline;
        pipeline
            .validate_config(&serde_json::json!({}))
            .await
            .expect("should accept empty config");
    }

    #[tokio::test]
    async fn prompt_score_crud() {
        let db = jarvis_daemon_common::DaemonDb::open_memory()
            .await
            .expect("open db");

        let pipe = jarvis_daemon_common::CreatePipeline {
            id: "seo-1".to_string(),
            name: "SEO".to_string(),
            strategy: jarvis_daemon_common::Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&pipe).await.expect("create pipeline");
        let job = db.create_job("seo-1").await.expect("create job");
        let content = db
            .create_content(
                &job.id,
                "seo-1",
                &jarvis_daemon_common::ContentOutput {
                    content_type: jarvis_daemon_common::ContentType::Article,
                    platform: jarvis_daemon_common::Platform::Wordpress,
                    title: "Test".to_string(),
                    slug: "test".to_string(),
                    body: "body".to_string(),
                    url: None,
                    word_count: Some(100),
                    llm_model: "test".to_string(),
                    llm_tokens_used: 10,
                    llm_cost_usd: None,
                },
            )
            .await
            .expect("create content");

        // Create score.
        let score = db
            .create_prompt_score(&CreatePromptScore {
                pipeline_id: "seo-1".to_string(),
                content_id: content.id.clone(),
                prompt_hash: "abc123".to_string(),
                params_json: serde_json::json!({"niche": "tech"}),
            })
            .await
            .expect("create score");

        assert_eq!(score.prompt_hash, "abc123");
        assert_eq!(score.avg_ctr, 0.0);

        // Update metrics.
        db.update_prompt_score_metrics(&score.id, 3.5, 150, 4285, 12.50)
            .await
            .expect("update metrics");

        // Fetch by content.
        let fetched = db
            .get_prompt_score_by_content(&content.id)
            .await
            .expect("get")
            .expect("exists");
        assert!((fetched.avg_ctr - 3.5).abs() < 0.01);
        assert_eq!(fetched.total_clicks, 150);
        assert_eq!(fetched.total_impressions, 4285);
        assert!((fetched.revenue_usd - 12.50).abs() < 0.01);
        assert!(fetched.composite_score > 0.0);

        // List scores.
        let scores = db.list_prompt_scores("seo-1").await.expect("list");
        assert_eq!(scores.len(), 1);

        // Performance summary.
        let summaries = db
            .prompt_performance_summary("seo-1")
            .await
            .expect("summary");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].content_count, 1);
        assert_eq!(summaries[0].prompt_hash, "abc123");
    }
}
