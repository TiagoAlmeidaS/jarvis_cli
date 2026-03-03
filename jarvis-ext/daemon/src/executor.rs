//! Proposal Executor — closes the autonomous loop by executing approved proposals.
//!
//! The executor reads proposals with status `approved` and applies the corresponding
//! actions to the system (create/modify pipelines, add/remove sources, etc.).

use anyhow::Context;
use anyhow::Result;
use jarvis_daemon_common::ActionType;
use jarvis_daemon_common::CreatePipeline;
use jarvis_daemon_common::CreateSource;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::DaemonProposal;
use jarvis_daemon_common::LogLevel;
use jarvis_daemon_common::ProposalFilter;
use jarvis_daemon_common::ProposalStatus;
use jarvis_daemon_common::SourceType;
use jarvis_daemon_common::Strategy;
use std::sync::Arc;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Summary of a proposal execution batch.
#[derive(Debug, Default)]
pub struct ExecutionSummary {
    pub executed: u64,
    pub failed: u64,
    pub skipped: u64,
}

/// Executes approved proposals by applying their actions to the system.
pub struct ProposalExecutor {
    db: Arc<DaemonDb>,
}

impl ProposalExecutor {
    pub fn new(db: Arc<DaemonDb>) -> Self {
        Self { db }
    }

    /// Check for approved proposals and execute them. Also expire stale proposals.
    pub async fn execute_pending(&self) -> Result<ExecutionSummary> {
        let mut summary = ExecutionSummary::default();

        // Expire stale proposals first.
        let expired = self.db.expire_proposals().await?;
        if expired > 0 {
            info!("{expired} proposals expired");
        }

        // Fetch approved proposals.
        let filter = ProposalFilter {
            status: Some(ProposalStatus::Approved),
            ..Default::default()
        };
        let proposals = self.db.list_proposals(&filter).await?;

        if proposals.is_empty() {
            return Ok(summary);
        }

        info!("{} approved proposals ready for execution", proposals.len());

        for proposal in &proposals {
            // Skip if already expired (edge case: approved after expiry window).
            if let Some(expires_at) = proposal.expires_at {
                if chrono::Utc::now().timestamp() > expires_at {
                    summary.skipped += 1;
                    continue;
                }
            }

            match self.execute_proposal(proposal).await {
                Ok(()) => {
                    self.db.mark_proposal_executed(&proposal.id).await?;
                    self.db
                        .insert_log(
                            proposal.pipeline_id.as_deref().unwrap_or("system"),
                            None,
                            LogLevel::Info,
                            &format!("Proposal executed: {} ({})", proposal.title, proposal.id),
                            None,
                        )
                        .await?;
                    info!(
                        "Proposal executed: {} ({}) [{}]",
                        proposal.title, proposal.action_type, proposal.id
                    );
                    summary.executed += 1;
                }
                Err(e) => {
                    let err_msg = format!("{e:#}");
                    if let Err(db_err) = self.db.mark_proposal_failed(&proposal.id).await {
                        error!("Failed to mark proposal as failed: {db_err}");
                    }
                    self.db
                        .insert_log(
                            proposal.pipeline_id.as_deref().unwrap_or("system"),
                            None,
                            LogLevel::Error,
                            &format!("Proposal execution failed: {} - {err_msg}", proposal.title),
                            None,
                        )
                        .await?;
                    error!(
                        "Proposal failed: {} ({}) - {err_msg}",
                        proposal.title, proposal.id
                    );
                    summary.failed += 1;
                }
            }
        }

        Ok(summary)
    }

    /// Execute a single proposal based on its action_type.
    async fn execute_proposal(&self, proposal: &DaemonProposal) -> Result<()> {
        let action: ActionType = proposal
            .action_type
            .parse()
            .context("invalid action_type")?;

        match action {
            ActionType::CreatePipeline => self.exec_create_pipeline(proposal).await,
            ActionType::ModifyPipeline => self.exec_modify_pipeline(proposal).await,
            ActionType::DisablePipeline => self.exec_disable_pipeline(proposal).await,
            ActionType::ChangeFrequency => self.exec_change_frequency(proposal).await,
            ActionType::ChangeNiche => self.exec_change_niche(proposal).await,
            ActionType::AddSource => self.exec_add_source(proposal).await,
            ActionType::RemoveSource => self.exec_remove_source(proposal).await,
            ActionType::ScaleUp => self.exec_scale_up(proposal).await,
            ActionType::ScaleDown => self.exec_scale_down(proposal).await,
            ActionType::ChangeModel => self.exec_change_model(proposal).await,
            ActionType::Custom => self.exec_custom(proposal).await,
        }
    }

    /// Parse the proposed_config JSON from a proposal.
    fn get_config(proposal: &DaemonProposal) -> Result<serde_json::Value> {
        let config_str = proposal.proposed_config.as_deref().unwrap_or("{}");
        Ok(serde_json::from_str(config_str)?)
    }

    /// Get the required pipeline_id from a proposal, or error.
    fn require_pipeline_id(proposal: &DaemonProposal) -> Result<&str> {
        proposal
            .pipeline_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("proposal requires a pipeline_id"))
    }

    // -----------------------------------------------------------------------
    // Action implementations
    // -----------------------------------------------------------------------

    async fn exec_create_pipeline(&self, proposal: &DaemonProposal) -> Result<()> {
        let config = Self::get_config(proposal)?;
        let id = config["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("proposed_config missing 'id'"))?;
        let name = config["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("proposed_config missing 'name'"))?;
        let strategy_str = config["strategy"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("proposed_config missing 'strategy'"))?;
        let strategy: Strategy = strategy_str.parse()?;
        let schedule = config["schedule_cron"].as_str().unwrap_or("0 3 * * *");
        let pipeline_config = config
            .get("config_json")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let input = CreatePipeline {
            id: id.to_string(),
            name: name.to_string(),
            strategy,
            config_json: pipeline_config,
            schedule_cron: schedule.to_string(),
            max_retries: config["max_retries"].as_i64().map(|v| v as i32),
            retry_delay_sec: config["retry_delay_sec"].as_i64().map(|v| v as i32),
        };

        self.db.create_pipeline(&input).await?;
        info!("Created pipeline: {id} ({name})");
        Ok(())
    }

    async fn exec_modify_pipeline(&self, proposal: &DaemonProposal) -> Result<()> {
        let pid = Self::require_pipeline_id(proposal)?;
        let config = Self::get_config(proposal)?;

        // Get existing pipeline to merge configs.
        let existing = self
            .db
            .get_pipeline(pid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("pipeline '{pid}' not found"))?;
        let mut existing_config: serde_json::Value = serde_json::from_str(&existing.config_json)?;

        // Merge proposed config into existing.
        if let Some(obj) = config.as_object() {
            if let Some(existing_obj) = existing_config.as_object_mut() {
                for (k, v) in obj {
                    existing_obj.insert(k.clone(), v.clone());
                }
            }
        }

        self.db
            .update_pipeline_config(pid, &existing_config)
            .await?;
        info!("Modified pipeline config: {pid}");
        Ok(())
    }

    async fn exec_disable_pipeline(&self, proposal: &DaemonProposal) -> Result<()> {
        let pid = Self::require_pipeline_id(proposal)?;
        self.db.set_pipeline_enabled(pid, false).await?;
        info!("Disabled pipeline: {pid}");
        Ok(())
    }

    async fn exec_change_frequency(&self, proposal: &DaemonProposal) -> Result<()> {
        let pid = Self::require_pipeline_id(proposal)?;
        let config = Self::get_config(proposal)?;
        let cron = config["schedule_cron"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("proposed_config missing 'schedule_cron'"))?;
        self.db.update_pipeline_schedule(pid, cron).await?;
        info!("Changed pipeline schedule: {pid} -> {cron}");
        Ok(())
    }

    async fn exec_change_niche(&self, proposal: &DaemonProposal) -> Result<()> {
        // Change niche is a modify_pipeline on the SEO config fields.
        self.exec_modify_pipeline(proposal).await
    }

    async fn exec_add_source(&self, proposal: &DaemonProposal) -> Result<()> {
        let pid = Self::require_pipeline_id(proposal)?;
        let config = Self::get_config(proposal)?;

        let source_type_str = config["source_type"].as_str().unwrap_or("rss");
        let source_type: SourceType = match source_type_str {
            "rss" => SourceType::Rss,
            "webpage" => SourceType::Webpage,
            "api" => SourceType::Api,
            "pdf_url" => SourceType::PdfUrl,
            "youtube_channel" => SourceType::YoutubeChannel,
            other => anyhow::bail!("unknown source_type: {other}"),
        };

        let name = config["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("proposed_config missing 'name'"))?;
        let url = config["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("proposed_config missing 'url'"))?;

        let input = CreateSource {
            pipeline_id: pid.to_string(),
            source_type,
            name: name.to_string(),
            url: url.to_string(),
            scrape_selector: config["scrape_selector"].as_str().map(String::from),
            check_interval_sec: config["check_interval_sec"].as_i64().map(|v| v as i32),
        };

        self.db.create_source(&input).await?;
        info!("Added source to pipeline {pid}: {name} ({url})");
        Ok(())
    }

    async fn exec_remove_source(&self, proposal: &DaemonProposal) -> Result<()> {
        let config = Self::get_config(proposal)?;
        let source_id = config["source_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("proposed_config missing 'source_id'"))?;
        self.db.delete_source(source_id).await?;
        info!("Removed source: {source_id}");
        Ok(())
    }

    async fn exec_scale_up(&self, proposal: &DaemonProposal) -> Result<()> {
        // Scale up: increase frequency via schedule change, or modify config.
        let config = Self::get_config(proposal)?;
        if config.get("schedule_cron").is_some() {
            self.exec_change_frequency(proposal).await
        } else {
            self.exec_modify_pipeline(proposal).await
        }
    }

    async fn exec_scale_down(&self, proposal: &DaemonProposal) -> Result<()> {
        let config = Self::get_config(proposal)?;
        if config.get("schedule_cron").is_some() {
            self.exec_change_frequency(proposal).await
        } else if config
            .get("disable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            self.exec_disable_pipeline(proposal).await
        } else {
            self.exec_modify_pipeline(proposal).await
        }
    }

    async fn exec_change_model(&self, proposal: &DaemonProposal) -> Result<()> {
        // Change model is a config modification on the llm section.
        self.exec_modify_pipeline(proposal).await
    }

    async fn exec_custom(&self, proposal: &DaemonProposal) -> Result<()> {
        // Custom proposals just log the description — they require manual action.
        warn!(
            "Custom proposal acknowledged: {} — manual action may be required",
            proposal.title
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jarvis_daemon_common::CreatePipeline;
    use jarvis_daemon_common::CreateProposal;
    use jarvis_daemon_common::CreateSource;
    use jarvis_daemon_common::RiskLevel;
    use jarvis_daemon_common::SourceType;
    use jarvis_daemon_common::Strategy;
    use pretty_assertions::assert_eq;

    /// Helper: create a DB + executor + a base pipeline for tests.
    async fn setup() -> (Arc<DaemonDb>, ProposalExecutor) {
        let db = Arc::new(DaemonDb::open_memory().await.expect("open db"));
        let executor = ProposalExecutor::new(db.clone());

        // Create a base pipeline that proposals can reference.
        let input = CreatePipeline {
            id: "seo-blog-1".to_string(),
            name: "SEO Blog".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({"niche": "tech", "articles_per_day": 3}),
            schedule_cron: "0 3 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&input).await.expect("create pipeline");

        (db, executor)
    }

    /// Helper: create and approve a proposal, then execute.
    async fn create_approve_execute(
        db: &DaemonDb,
        executor: &ProposalExecutor,
        action_type: ActionType,
        pipeline_id: Option<String>,
        proposed_config: Option<serde_json::Value>,
    ) -> ExecutionSummary {
        let input = CreateProposal {
            pipeline_id,
            action_type,
            title: format!("Test {action_type}"),
            description: "Test description".to_string(),
            reasoning: "Test reasoning".to_string(),
            confidence: 0.9,
            risk_level: RiskLevel::Low,
            proposed_config,
            metrics_snapshot: None,
            auto_approvable: false,
            expires_in_hours: Some(24),
        };
        let proposal = db.create_proposal(&input).await.expect("create proposal");
        db.approve_proposal(&proposal.id)
            .await
            .expect("approve proposal");

        executor.execute_pending().await.expect("execute_pending")
    }

    #[tokio::test]
    async fn test_create_pipeline_action() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::CreatePipeline,
            None,
            Some(serde_json::json!({
                "id": "new-pipe",
                "name": "New Pipeline",
                "strategy": "seo_blog",
                "schedule_cron": "0 6 * * *",
                "config_json": {"niche": "finance"}
            })),
        )
        .await;

        assert_eq!(summary.executed, 1);
        assert_eq!(summary.failed, 0);

        // Verify pipeline was created.
        let pipe = db
            .get_pipeline("new-pipe")
            .await
            .expect("get")
            .expect("pipeline should exist");
        assert_eq!(pipe.name, "New Pipeline");
        assert_eq!(pipe.schedule_cron, "0 6 * * *");
    }

    #[tokio::test]
    async fn test_modify_pipeline_action() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::ModifyPipeline,
            Some("seo-blog-1".to_string()),
            Some(serde_json::json!({"articles_per_day": 5, "new_field": "value"})),
        )
        .await;

        assert_eq!(summary.executed, 1);

        // Verify config was merged (not replaced).
        let pipe = db
            .get_pipeline("seo-blog-1")
            .await
            .expect("get")
            .expect("exists");
        let config: serde_json::Value =
            serde_json::from_str(&pipe.config_json).expect("parse config");
        assert_eq!(config["niche"], "tech"); // Original field preserved.
        assert_eq!(config["articles_per_day"], 5); // Updated.
        assert_eq!(config["new_field"], "value"); // New field added.
    }

    #[tokio::test]
    async fn test_disable_pipeline_action() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::DisablePipeline,
            Some("seo-blog-1".to_string()),
            None,
        )
        .await;

        assert_eq!(summary.executed, 1);

        let pipe = db
            .get_pipeline("seo-blog-1")
            .await
            .expect("get")
            .expect("exists");
        assert!(!pipe.enabled);
    }

    #[tokio::test]
    async fn test_change_frequency_action() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::ChangeFrequency,
            Some("seo-blog-1".to_string()),
            Some(serde_json::json!({"schedule_cron": "0 */4 * * *"})),
        )
        .await;

        assert_eq!(summary.executed, 1);

        let pipe = db
            .get_pipeline("seo-blog-1")
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(pipe.schedule_cron, "0 */4 * * *");
    }

    #[tokio::test]
    async fn test_add_source_action() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::AddSource,
            Some("seo-blog-1".to_string()),
            Some(serde_json::json!({
                "source_type": "rss",
                "name": "Tech News RSS",
                "url": "https://example.com/feed.xml",
                "check_interval_sec": 7200
            })),
        )
        .await;

        assert_eq!(summary.executed, 1);

        let sources = db.list_sources("seo-blog-1").await.expect("list sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "Tech News RSS");
        assert_eq!(sources[0].url, "https://example.com/feed.xml");
    }

    #[tokio::test]
    async fn test_remove_source_action() {
        let (db, executor) = setup().await;

        // First add a source.
        let source_input = CreateSource {
            pipeline_id: "seo-blog-1".to_string(),
            source_type: SourceType::Rss,
            name: "To Remove".to_string(),
            url: "https://example.com/rss".to_string(),
            scrape_selector: None,
            check_interval_sec: None,
        };
        let source = db.create_source(&source_input).await.expect("create");

        // Now propose to remove it.
        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::RemoveSource,
            Some("seo-blog-1".to_string()),
            Some(serde_json::json!({"source_id": source.id})),
        )
        .await;

        assert_eq!(summary.executed, 1);

        let sources = db.list_sources("seo-blog-1").await.expect("list");
        assert!(sources.is_empty());
    }

    #[tokio::test]
    async fn test_custom_action_succeeds() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::Custom,
            None,
            Some(serde_json::json!({"note": "Manual action needed"})),
        )
        .await;

        // Custom always succeeds (just logs).
        assert_eq!(summary.executed, 1);
        assert_eq!(summary.failed, 0);
    }

    #[tokio::test]
    async fn test_no_approved_proposals_returns_empty_summary() {
        let (db, executor) = setup().await;

        let summary = executor.execute_pending().await.expect("execute_pending");
        assert_eq!(summary.executed, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[tokio::test]
    async fn test_proposal_marked_executed_after_success() {
        let (db, executor) = setup().await;

        let input = CreateProposal {
            pipeline_id: Some("seo-blog-1".to_string()),
            action_type: ActionType::DisablePipeline,
            title: "Disable it".to_string(),
            description: "Test".to_string(),
            reasoning: "Test".to_string(),
            confidence: 0.9,
            risk_level: RiskLevel::Low,
            proposed_config: None,
            metrics_snapshot: None,
            auto_approvable: false,
            expires_in_hours: None,
        };
        let proposal = db.create_proposal(&input).await.expect("create");
        db.approve_proposal(&proposal.id).await.expect("approve");

        executor.execute_pending().await.expect("execute");

        // Verify the proposal status is now 'executed'.
        let fetched = db
            .get_proposal(&proposal.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(fetched.status, "executed");
        assert!(fetched.executed_at.is_some());
    }

    #[tokio::test]
    async fn test_proposal_marked_failed_on_error() {
        let (db, executor) = setup().await;

        // Propose to change frequency but provide an INVALID config (missing schedule_cron).
        // The pipeline exists, so the FK constraint is satisfied, but the executor
        // will fail because it can't find "schedule_cron" in the proposed config.
        let input = CreateProposal {
            pipeline_id: Some("seo-blog-1".to_string()),
            action_type: ActionType::ChangeFrequency,
            title: "Bad frequency change".to_string(),
            description: "This should fail due to missing schedule_cron".to_string(),
            reasoning: "Test".to_string(),
            confidence: 0.5,
            risk_level: RiskLevel::Low,
            proposed_config: Some(serde_json::json!({"invalid_key": "no_cron_here"})),
            metrics_snapshot: None,
            auto_approvable: false,
            expires_in_hours: None,
        };
        let proposal = db.create_proposal(&input).await.expect("create");
        db.approve_proposal(&proposal.id).await.expect("approve");

        let summary = executor.execute_pending().await.expect("execute");
        assert_eq!(summary.failed, 1);

        // Verify the proposal status is now 'failed'.
        let fetched = db
            .get_proposal(&proposal.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(fetched.status, "failed");
    }

    #[tokio::test]
    async fn test_scale_up_with_schedule_changes_frequency() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::ScaleUp,
            Some("seo-blog-1".to_string()),
            Some(serde_json::json!({"schedule_cron": "0 */2 * * *"})),
        )
        .await;

        assert_eq!(summary.executed, 1);

        let pipe = db
            .get_pipeline("seo-blog-1")
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(pipe.schedule_cron, "0 */2 * * *");
    }

    #[tokio::test]
    async fn test_scale_up_without_schedule_modifies_config() {
        let (db, executor) = setup().await;

        let summary = create_approve_execute(
            &db,
            &executor,
            ActionType::ScaleUp,
            Some("seo-blog-1".to_string()),
            Some(serde_json::json!({"articles_per_day": 10})),
        )
        .await;

        assert_eq!(summary.executed, 1);

        let pipe = db
            .get_pipeline("seo-blog-1")
            .await
            .expect("get")
            .expect("exists");
        let config: serde_json::Value = serde_json::from_str(&pipe.config_json).expect("parse");
        assert_eq!(config["articles_per_day"], 10);
    }
}
