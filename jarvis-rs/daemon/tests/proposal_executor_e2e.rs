//! E2E integration test: approved proposal → executor → state change.
//!
//! Validates that the loop "Strategy Analyzer → Proposals → Executor → Action"
//! closes: an approved proposal is executed and the system state is updated.
//! See issue #34 and docs/features/proposal-executor.md.

use anyhow::Result;
use pretty_assertions::assert_eq;
use std::sync::Arc;

use jarvis_daemon::executor::ExecutionSummary;
use jarvis_daemon::executor::ProposalExecutor;
use jarvis_daemon_common::ActionType;
use jarvis_daemon_common::CreatePipeline;
use jarvis_daemon_common::CreateProposal;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::RiskLevel;
use jarvis_daemon_common::Strategy;

/// Full flow: create proposal → approve → execute → assert state.
///
/// Simulates what the strategy_analyzer (or operator) produces and what the
/// scheduler's executor runs each tick.
#[tokio::test]
async fn approved_proposal_create_pipeline_executed_and_state_updated() -> Result<()> {
    let db = Arc::new(DaemonDb::open_memory().await?);
    let executor = ProposalExecutor::new(db.clone());

    let proposal_input = CreateProposal {
        pipeline_id: None,
        action_type: ActionType::CreatePipeline,
        title: "Add pipeline: e2e-test-pipeline".to_string(),
        description: "E2E test proposal".to_string(),
        reasoning: "Validation for issue #34".to_string(),
        confidence: 0.9,
        risk_level: RiskLevel::Low,
        proposed_config: Some(serde_json::json!({
            "id": "e2e-test-pipeline",
            "name": "E2E Test Pipeline",
            "strategy": "seo_blog",
            "schedule_cron": "0 8 * * *",
            "config_json": {"niche": "Tecnologia"}
        })),
        metrics_snapshot: None,
        auto_approvable: false,
        expires_in_hours: Some(24),
    };

    let proposal = db.create_proposal(&proposal_input).await?;
    db.approve_proposal(&proposal.id).await?;

    let summary: ExecutionSummary = executor.execute_pending().await?;

    assert_eq!(summary.executed, 1, "one proposal should be executed");
    assert_eq!(summary.failed, 0, "no failures expected");
    assert_eq!(summary.skipped, 0);

    let pipeline = db
        .get_pipeline("e2e-test-pipeline")
        .await?
        .expect("pipeline should exist after CreatePipeline execution");
    assert_eq!(pipeline.name, "E2E Test Pipeline");
    assert_eq!(pipeline.schedule_cron, "0 8 * * *");
    assert_eq!(pipeline.strategy, "seo_blog");

    let fetched = db
        .get_proposal(&proposal.id)
        .await?
        .expect("proposal should exist");
    assert_eq!(fetched.status, "executed");
    assert!(fetched.executed_at.is_some());

    Ok(())
}

/// Same flow for DisablePipeline: existing pipeline → approved proposal → disabled.
#[tokio::test]
async fn approved_proposal_disable_pipeline_executed_and_state_updated() -> Result<()> {
    let db = Arc::new(DaemonDb::open_memory().await?);

    let pipe_input = CreatePipeline {
        id: "to-disable".to_string(),
        name: "To Disable".to_string(),
        strategy: Strategy::SeoBlog,
        config_json: serde_json::json!({}),
        schedule_cron: "0 3 * * *".to_string(),
        max_retries: None,
        retry_delay_sec: None,
    };
    db.create_pipeline(&pipe_input).await?;

    let executor = ProposalExecutor::new(db.clone());
    let proposal_input = CreateProposal {
        pipeline_id: Some("to-disable".to_string()),
        action_type: ActionType::DisablePipeline,
        title: "Disable pipeline to-disable".to_string(),
        description: "E2E disable test".to_string(),
        reasoning: "Validation for issue #34".to_string(),
        confidence: 0.9,
        risk_level: RiskLevel::Low,
        proposed_config: None,
        metrics_snapshot: None,
        auto_approvable: false,
        expires_in_hours: Some(24),
    };

    let proposal = db.create_proposal(&proposal_input).await?;
    db.approve_proposal(&proposal.id).await?;

    let summary: ExecutionSummary = executor.execute_pending().await?;

    assert_eq!(summary.executed, 1);
    assert_eq!(summary.failed, 0);

    let pipeline = db.get_pipeline("to-disable").await?.expect("pipeline exists");
    assert!(!pipeline.enabled, "pipeline should be disabled");

    let fetched = db.get_proposal(&proposal.id).await?.expect("proposal exists");
    assert_eq!(fetched.status, "executed");

    Ok(())
}
