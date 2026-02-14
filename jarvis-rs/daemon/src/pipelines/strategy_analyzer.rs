//! Strategy Analyzer pipeline — Phase 4: Analysis + Decision.
//!
//! Periodically analyzes collected metrics and proposes actions:
//! 1. Loads metrics and revenue data from the database
//! 2. Builds a structured analysis prompt
//! 3. Sends to LLM for evaluation
//! 4. Parses LLM response into ProposedAction records
//! 5. Stores proposals in daemon_proposals table
//!
//! Proposals with high confidence + low risk can be auto-approved.

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{
    ActionType, ContentFilter, ContentOutput, CreateProposal, GoalFilter, GoalStatus, LogLevel,
    ProposalFilter, ProposalStatus, RiskLevel,
};
use serde::{Deserialize, Serialize};

use crate::pipeline::{Pipeline, PipelineContext};
use crate::processor;

/// Strategy Analyzer pipeline implementation.
pub struct StrategyAnalyzerPipeline;

/// Configuration for the strategy analyzer pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct StrategyAnalyzerConfig {
    /// LLM configuration for analysis.
    #[serde(default)]
    pub llm: processor::LlmConfig,
    /// Number of days to analyze.
    #[serde(default = "default_analysis_window")]
    pub analysis_window_days: i64,
    /// Minimum confidence (0.0-1.0) for auto-approval.
    #[serde(default = "default_min_confidence")]
    pub min_confidence_for_auto_approve: f64,
    /// Maximum risk level for auto-approval.
    #[serde(default = "default_max_risk")]
    pub max_auto_approve_risk: String,
    /// Maximum proposals to generate per run.
    #[serde(default = "default_max_proposals")]
    pub max_proposals_per_run: usize,
}

impl Default for StrategyAnalyzerConfig {
    fn default() -> Self {
        Self {
            llm: processor::LlmConfig::default(),
            analysis_window_days: default_analysis_window(),
            min_confidence_for_auto_approve: default_min_confidence(),
            max_auto_approve_risk: default_max_risk(),
            max_proposals_per_run: default_max_proposals(),
        }
    }
}

fn default_analysis_window() -> i64 {
    30
}

fn default_min_confidence() -> f64 {
    0.85
}

fn default_max_risk() -> String {
    "low".to_string()
}

fn default_max_proposals() -> usize {
    5
}

/// Structure the LLM is asked to return.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmProposal {
    pub action_type: String,
    pub title: String,
    pub description: String,
    pub reasoning: String,
    pub confidence: f64,
    pub risk_level: String,
    #[serde(default)]
    pub pipeline_id: Option<String>,
    #[serde(default)]
    pub proposed_config: Option<serde_json::Value>,
}

#[async_trait]
impl Pipeline for StrategyAnalyzerPipeline {
    fn strategy(&self) -> &str {
        "strategy_analyzer"
    }

    fn display_name(&self) -> &str {
        "Strategy Analyzer"
    }

    async fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _parsed: StrategyAnalyzerConfig = serde_json::from_value(config.clone())?;
        Ok(())
    }

    async fn execute(&self, ctx: &PipelineContext) -> Result<Vec<ContentOutput>> {
        let config: StrategyAnalyzerConfig = ctx.pipeline.config()?;
        let db = &ctx.db;

        ctx.log_info(&format!(
            "Starting strategy analysis (window: {}d)",
            config.analysis_window_days
        ))
        .await;

        // 1. Gather data for analysis.
        let analysis_data = gather_analysis_data(ctx, &config).await?;

        if analysis_data.is_empty() {
            ctx.log_info("No data available for analysis, skipping.")
                .await;
            return Ok(vec![]);
        }

        // 2. Check if there are already too many pending proposals.
        let pending_count = db.count_pending_proposals().await?;
        if pending_count >= config.max_proposals_per_run as i64 {
            ctx.log_info(&format!(
                "Already {pending_count} pending proposals, skipping analysis to avoid overload",
            ))
            .await;
            return Ok(vec![]);
        }

        // 3. Build the analysis prompt.
        let system_prompt = build_system_prompt();
        let user_prompt = build_user_prompt(&analysis_data, &config);

        ctx.log(LogLevel::Debug, "Sending analysis to LLM").await;

        // 4. Call LLM for analysis.
        let llm_response = ctx
            .llm_client
            .generate_json(&user_prompt, Some(&system_prompt))
            .await;

        let proposals_json = match llm_response {
            Ok(json) => json,
            Err(e) => {
                ctx.log_error(&format!("LLM analysis failed: {e}")).await;
                return Ok(vec![]);
            }
        };

        // 5. Parse proposals from LLM response.
        let raw_proposals = parse_proposals(&proposals_json);

        ctx.log_info(&format!("LLM suggested {} proposals", raw_proposals.len()))
            .await;

        // 6. Create proposals in the database.
        let max_risk = parse_risk_level(&config.max_auto_approve_risk);
        let mut created_count = 0;

        for raw in raw_proposals.into_iter().take(config.max_proposals_per_run) {
            let action_type = match parse_action_type(&raw.action_type) {
                Some(at) => at,
                None => {
                    ctx.log(
                        LogLevel::Warn,
                        &format!("Unknown action type '{}', skipping", raw.action_type),
                    )
                    .await;
                    continue;
                }
            };

            let risk = parse_risk_level(&raw.risk_level);
            let confidence = raw.confidence.clamp(0.0, 1.0);

            let auto_approvable = confidence >= config.min_confidence_for_auto_approve
                && risk_level_value(risk) <= risk_level_value(max_risk);

            let input = CreateProposal {
                pipeline_id: raw.pipeline_id.clone(),
                action_type,
                title: raw.title.clone(),
                description: raw.description.clone(),
                reasoning: raw.reasoning.clone(),
                confidence,
                risk_level: risk,
                proposed_config: raw.proposed_config.clone(),
                metrics_snapshot: Some(serde_json::Value::String(analysis_data.clone())),
                auto_approvable,
                expires_in_hours: Some(168), // 7 days
            };

            match db.create_proposal(&input).await {
                Ok(proposal) => {
                    created_count += 1;
                    let auto_label = if auto_approvable {
                        " [AUTO-APPROVABLE]"
                    } else {
                        ""
                    };
                    ctx.log_info(&format!(
                        "Proposal created: {} (confidence: {:.0}%, risk: {}){auto_label}",
                        proposal.title,
                        confidence * 100.0,
                        proposal.risk_level,
                    ))
                    .await;

                    // Auto-approve if eligible.
                    if auto_approvable {
                        if let Err(e) = db.approve_proposal(&proposal.id).await {
                            ctx.log(
                                LogLevel::Warn,
                                &format!("Failed to auto-approve proposal: {e}"),
                            )
                            .await;
                        } else {
                            ctx.log_info(&format!("Proposal auto-approved: {}", proposal.title))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    ctx.log_error(&format!("Failed to create proposal: {e}"))
                        .await;
                }
            }
        }

        ctx.log_info(&format!(
            "Strategy analysis complete: {created_count} proposals created",
        ))
        .await;

        // Strategy analyzer doesn't produce content outputs.
        Ok(vec![])
    }
}

/// Gather all relevant data for the LLM analysis.
async fn gather_analysis_data(
    ctx: &PipelineContext,
    config: &StrategyAnalyzerConfig,
) -> Result<String> {
    let db = &ctx.db;

    let mut data = String::new();

    // Pipelines overview.
    let pipelines = db.list_pipelines(false).await?;
    data.push_str("## Pipelines\n");
    for p in &pipelines {
        data.push_str(&format!(
            "- {} ({}): strategy={}, enabled={}, schedule={}\n",
            p.name, p.id, p.strategy, p.enabled, p.schedule_cron
        ));
    }

    // Content summary per pipeline.
    data.push_str("\n## Content (last ");
    data.push_str(&config.analysis_window_days.to_string());
    data.push_str(" days)\n");

    for p in &pipelines {
        let content_filter = ContentFilter {
            pipeline_id: Some(p.id.clone()),
            since_days: Some(config.analysis_window_days),
            ..Default::default()
        };
        let content = db.list_content(&content_filter).await?;
        let published = content.iter().filter(|c| c.status == "published").count();
        let failed = content.iter().filter(|c| c.status == "failed").count();
        let total_cost: f64 = content.iter().filter_map(|c| c.llm_cost_usd).sum();

        data.push_str(&format!(
            "- Pipeline '{}': {} total, {} published, {} failed, LLM cost: ${:.4}\n",
            p.id,
            content.len(),
            published,
            failed,
            total_cost
        ));
    }

    // Revenue summary.
    let revenue = db.revenue_summary(config.analysis_window_days).await?;
    data.push_str(&format!(
        "\n## Revenue (last {}d)\n",
        config.analysis_window_days
    ));
    data.push_str(&format!("Total: ${:.2}\n", revenue.total_usd));
    for pr in &revenue.by_pipeline {
        let name = pr.pipeline_name.as_deref().unwrap_or(&pr.pipeline_id);
        data.push_str(&format!(
            "- {}: ${:.2} ({} records)\n",
            name, pr.total_usd, pr.content_count
        ));
    }

    // Active goals.
    let goals = db
        .list_goals(&GoalFilter {
            status: Some(GoalStatus::Active),
            ..Default::default()
        })
        .await?;
    if !goals.is_empty() {
        data.push_str("\n## Active Goals (PRIORITIZE actions that advance these)\n");
        for g in &goals {
            let progress_pct = if g.target_value > 0.0 {
                (g.current_value / g.target_value) * 100.0
            } else {
                0.0
            };
            let gap = g.target_value - g.current_value;
            let status_label = if progress_pct >= 100.0 {
                "ACHIEVED"
            } else if progress_pct >= 60.0 {
                "ON TRACK"
            } else {
                "AT RISK"
            };
            data.push_str(&format!(
                "- P{} {} ({}): {:.1}/{:.1} {} ({:.0}% — {})\n",
                g.priority,
                g.name,
                g.metric_type,
                g.current_value,
                g.target_value,
                g.target_unit,
                progress_pct,
                status_label,
            ));
            if gap > 0.0 {
                data.push_str(&format!("  Gap: {:.2} {}\n", gap, g.target_unit));
            }
        }
    }

    // Existing pending proposals (to avoid duplicates).
    let pending_filter = ProposalFilter {
        status: Some(ProposalStatus::Pending),
        ..Default::default()
    };
    let pending = db.list_proposals(&pending_filter).await?;
    if !pending.is_empty() {
        data.push_str("\n## Pending Proposals (do not duplicate)\n");
        for prop in &pending {
            data.push_str(&format!("- {}: {}\n", prop.action_type, prop.title));
        }
    }

    Ok(data)
}

fn build_system_prompt() -> String {
    String::from(
        "You are a strategic automation advisor for an autonomous content pipeline system. \
         Your role is to analyze performance metrics and suggest actionable improvements.\n\n\
         RULES:\n\
         - Be conservative with high-risk suggestions\n\
         - Focus on maximizing revenue with minimal cost\n\
         - Consider the current state of pending proposals to avoid duplicates\n\
         - Each proposal must have a clear, measurable outcome\n\
         - Confidence should reflect how certain you are the action will improve results\n\
         - If Active Goals are listed, PRIORITIZE actions that close the largest gaps\n\
         - If a goal is AT RISK, suggest urgent corrective actions\n\
         - Never suggest actions that conflict with higher-priority goals\n\n\
         RESPOND IN JSON ONLY. Return an array of proposals:\n\
         ```json\n\
         {\n\
           \"proposals\": [\n\
             {\n\
               \"action_type\": \"scale_up|scale_down|change_frequency|change_niche|add_source|change_model|create_pipeline|modify_pipeline|disable_pipeline|custom\",\n\
               \"title\": \"Short action title\",\n\
               \"description\": \"Detailed description of the proposed change\",\n\
               \"reasoning\": \"Chain of thought explaining why this is a good idea based on data\",\n\
               \"confidence\": 0.85,\n\
               \"risk_level\": \"low|medium|high\",\n\
               \"pipeline_id\": \"optional-pipeline-id\",\n\
               \"proposed_config\": {}\n\
             }\n\
           ]\n\
         }\n\
         ```\n\
         If no improvements are needed, return {\"proposals\": []}.",
    )
}

fn build_user_prompt(analysis_data: &str, config: &StrategyAnalyzerConfig) -> String {
    format!(
        "Analyze the following performance data and suggest up to {} improvements.\n\n\
         Auto-approval threshold: confidence >= {:.0}%, risk <= {}\n\n\
         DATA:\n{analysis_data}",
        config.max_proposals_per_run,
        config.min_confidence_for_auto_approve * 100.0,
        config.max_auto_approve_risk,
    )
}

fn parse_proposals(json: &serde_json::Value) -> Vec<LlmProposal> {
    // Try to extract from {"proposals": [...]} or just [...]
    let array = if let Some(arr) = json.get("proposals").and_then(|v| v.as_array()) {
        arr.clone()
    } else if let Some(arr) = json.as_array() {
        arr.clone()
    } else {
        return vec![];
    };

    array
        .into_iter()
        .filter_map(|v| serde_json::from_value::<LlmProposal>(v).ok())
        .collect()
}

fn parse_action_type(s: &str) -> Option<ActionType> {
    match s {
        "create_pipeline" => Some(ActionType::CreatePipeline),
        "modify_pipeline" => Some(ActionType::ModifyPipeline),
        "disable_pipeline" => Some(ActionType::DisablePipeline),
        "change_niche" => Some(ActionType::ChangeNiche),
        "change_frequency" => Some(ActionType::ChangeFrequency),
        "add_source" => Some(ActionType::AddSource),
        "remove_source" => Some(ActionType::RemoveSource),
        "scale_up" => Some(ActionType::ScaleUp),
        "scale_down" => Some(ActionType::ScaleDown),
        "change_model" => Some(ActionType::ChangeModel),
        "custom" => Some(ActionType::Custom),
        _ => None,
    }
}

fn parse_risk_level(s: &str) -> RiskLevel {
    match s {
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        _ => RiskLevel::Medium,
    }
}

fn risk_level_value(r: RiskLevel) -> u8 {
    match r {
        RiskLevel::Low => 1,
        RiskLevel::Medium => 2,
        RiskLevel::High => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_config_defaults() {
        let config: StrategyAnalyzerConfig =
            serde_json::from_value(serde_json::json!({})).expect("parse empty config");
        assert_eq!(config.analysis_window_days, 30);
        assert!((config.min_confidence_for_auto_approve - 0.85).abs() < 0.001);
        assert_eq!(config.max_auto_approve_risk, "low");
    }

    #[test]
    fn parse_llm_proposals_response() {
        let json = serde_json::json!({
            "proposals": [
                {
                    "action_type": "scale_up",
                    "title": "Increase article frequency",
                    "description": "Publish 5 articles per day instead of 3",
                    "reasoning": "CTR is above average at 5.2%",
                    "confidence": 0.87,
                    "risk_level": "low",
                    "pipeline_id": "seo-concursos"
                },
                {
                    "action_type": "add_source",
                    "title": "Add new RSS feed",
                    "description": "Add governo.br RSS feed for more data",
                    "reasoning": "Current sources are running low on new content",
                    "confidence": 0.72,
                    "risk_level": "medium"
                }
            ]
        });

        let proposals = parse_proposals(&json);
        assert_eq!(proposals.len(), 2);
        assert_eq!(proposals[0].action_type, "scale_up");
        assert!((proposals[0].confidence - 0.87).abs() < 0.001);
        assert_eq!(proposals[1].risk_level, "medium");
    }

    #[test]
    fn parse_empty_proposals() {
        let json = serde_json::json!({"proposals": []});
        let proposals = parse_proposals(&json);
        assert!(proposals.is_empty());
    }

    #[test]
    fn risk_level_ordering() {
        assert!(risk_level_value(RiskLevel::Low) < risk_level_value(RiskLevel::Medium));
        assert!(risk_level_value(RiskLevel::Medium) < risk_level_value(RiskLevel::High));
    }

    #[test]
    fn system_prompt_includes_goal_rules() {
        let prompt = build_system_prompt();
        assert!(
            prompt.contains("PRIORITIZE actions that close the largest gaps"),
            "system prompt must instruct LLM to prioritize goals"
        );
        assert!(
            prompt.contains("AT RISK"),
            "system prompt must mention AT RISK corrective actions"
        );
        assert!(
            prompt.contains("higher-priority goals"),
            "system prompt must warn against conflicting with priorities"
        );
    }

    #[test]
    fn user_prompt_contains_data() {
        let config = StrategyAnalyzerConfig::default();
        let data = "## Active Goals\n- P1 Revenue $50 (revenue): 10.0/50.0 USD (20% — AT RISK)";
        let prompt = build_user_prompt(data, &config);
        assert!(prompt.contains("Active Goals"));
        assert!(prompt.contains("AT RISK"));
        assert!(prompt.contains("up to 5 improvements"));
    }

    #[test]
    fn auto_approve_logic() {
        // High confidence + low risk = auto-approvable
        let confidence = 0.90;
        let risk = RiskLevel::Low;
        let min_confidence = 0.85;
        let max_risk = RiskLevel::Low;
        assert!(
            confidence >= min_confidence && risk_level_value(risk) <= risk_level_value(max_risk)
        );

        // High confidence but medium risk when max is low = not auto-approvable
        let risk2 = RiskLevel::Medium;
        assert!(
            !(confidence >= min_confidence
                && risk_level_value(risk2) <= risk_level_value(max_risk))
        );
    }
}
