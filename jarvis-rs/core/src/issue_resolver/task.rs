//! `IssueResolverTask` — a [`SessionTask`] that drives the full issue-resolution
//! pipeline: scan -> analyze -> plan -> safety-gate -> execute.
//!
//! This is the entry-point for autonomous issue resolution, following the same
//! pattern as [`ReviewTask`](crate::tasks::review::ReviewTask).

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use jarvis_github::GitHubClient;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use crate::TurnContext;
use crate::state::TaskKind;
use jarvis_protocol::user_input::UserInput;

use super::analyzer::IssueAnalyzer;
use super::context::ContextBuilder;
use super::executor::ImplementationExecutor;
use super::planner::ImplementationPlanner;
use super::safety_gate;
use super::types::ExecutionResult;
use super::types::ExecutionStatus;
use super::types::IssueResolution;
use crate::safety::classifier::RuleBasedSafetyClassifier;
use crate::tasks::SessionTask;
use crate::tasks::SessionTaskContext;

/// Parameters for launching an issue-resolver task.
#[derive(Debug, Clone)]
pub struct IssueResolverParams {
    /// GitHub PAT for API access.
    pub github_pat: String,
    /// Repository owner (e.g., "openai").
    pub owner: String,
    /// Repository name (e.g., "codex").
    pub repo: String,
    /// Issue number to resolve.
    pub issue_number: u64,
}

/// Session task that runs the issue-resolver pipeline.
pub struct IssueResolverTask {
    params: IssueResolverParams,
}

impl IssueResolverTask {
    pub fn new(params: IssueResolverParams) -> Self {
        Self { params }
    }
}

#[async_trait]
impl SessionTask for IssueResolverTask {
    fn kind(&self) -> TaskKind {
        TaskKind::IssueResolver
    }

    async fn run(
        self: Arc<Self>,
        session: Arc<SessionTaskContext>,
        ctx: Arc<TurnContext>,
        _input: Vec<UserInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        let _ = session.clone_session().services.otel_manager.counter(
            "Jarvis.task.issue_resolver",
            1,
            &[],
        );

        let result = self
            .run_pipeline(session.clone(), ctx.clone(), cancellation_token.clone())
            .await;

        match result {
            Ok(resolution) => {
                let json = serde_json::to_string_pretty(&resolution)
                    .unwrap_or_else(|_| format!("{resolution:?}"));
                Some(json)
            }
            Err(e) => {
                warn!("issue resolver pipeline failed: {e}");
                Some(format!("Issue resolver error: {e}"))
            }
        }
    }
}

impl IssueResolverTask {
    async fn run_pipeline(
        &self,
        session: Arc<SessionTaskContext>,
        ctx: Arc<TurnContext>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<IssueResolution> {
        let p = &self.params;

        info!(
            "starting issue resolver pipeline for {}/{} #{}",
            p.owner, p.repo, p.issue_number
        );

        // 1. Build GitHub client.
        let gh_client = GitHubClient::new(p.github_pat.clone())
            .map_err(|e| anyhow::anyhow!("failed to create GitHub client: {e}"))?;

        // 2. Fetch issue + comments.
        let issue = gh_client
            .get_issue(&p.owner, &p.repo, p.issue_number)
            .await
            .map_err(|e| anyhow::anyhow!("failed to fetch issue: {e}"))?;

        let comments = gh_client
            .list_issue_comments(&p.owner, &p.repo, p.issue_number)
            .await
            .map_err(|e| anyhow::anyhow!("failed to fetch issue comments: {e}"))?;

        // 3. Build repo context.
        let context_builder = ContextBuilder::new(&gh_client, &p.owner, &p.repo);
        let repo_context = context_builder.build().await?;

        info!(
            "repo context built: language={:?}, framework={:?}, {} patterns",
            repo_context.language,
            repo_context.framework,
            repo_context.patterns.len()
        );

        // 4. Analyze issue using LLM sub-agent.
        if cancel_token.is_cancelled() {
            return Err(anyhow::anyhow!("cancelled before analysis"));
        }

        let config = ctx.config.as_ref().clone();
        let analysis = IssueAnalyzer::analyze(
            &config,
            session.auth_manager(),
            session.models_manager(),
            session.clone_session(),
            ctx.clone(),
            cancel_token.child_token(),
            &issue,
            &comments,
            &repo_context,
        )
        .await?;

        info!(
            "analysis complete: complexity={:?}, category={:?}, can_auto_resolve={}, confidence={}",
            analysis.complexity, analysis.category, analysis.can_auto_resolve, analysis.confidence
        );

        // 5. If not auto-resolvable, stop here.
        if !analysis.can_auto_resolve {
            return Ok(IssueResolution {
                issue_number: p.issue_number,
                repo_full_name: format!("{}/{}", p.owner, p.repo),
                analysis,
                plan: None,
                safety: None,
                should_proceed: false,
                rejection_reason: Some(
                    "Issue analysis determined this cannot be auto-resolved".to_string(),
                ),
                execution: None,
            });
        }

        // 6. Create implementation plan.
        if cancel_token.is_cancelled() {
            return Err(anyhow::anyhow!("cancelled before planning"));
        }

        let plan = ImplementationPlanner::plan(
            &config,
            session.auth_manager(),
            session.models_manager(),
            session.clone_session(),
            ctx.clone(),
            cancel_token.child_token(),
            &issue,
            &analysis,
            &repo_context,
        )
        .await?;

        info!(
            "plan complete: {} steps, branch={}, confidence={}",
            plan.steps.len(),
            plan.branch_name,
            plan.confidence
        );

        // 7. Safety assessment.
        let classifier = RuleBasedSafetyClassifier::default();
        let safety = safety_gate::assess_plan(&classifier, &plan).await?;

        info!(
            "safety assessment: is_safe={}, risk_level={:?}, requires_human_review={}",
            safety.is_safe, safety.risk_level, safety.requires_human_review
        );

        let should_proceed = safety.is_safe && !safety.requires_human_review;

        let rejection_reason = if !should_proceed {
            Some(format!(
                "Safety gate: risk_level={:?}, requires_human_review={}",
                safety.risk_level, safety.requires_human_review
            ))
        } else {
            None
        };

        if !should_proceed {
            return Ok(IssueResolution {
                issue_number: p.issue_number,
                repo_full_name: format!("{}/{}", p.owner, p.repo),
                analysis,
                plan: Some(plan),
                safety: Some(safety),
                should_proceed: false,
                rejection_reason,
                execution: None,
            });
        }

        let plan = plan;

        info!("safety gate passed, proceeding to execution");

        if cancel_token.is_cancelled() {
            return Err(anyhow::anyhow!("cancelled before execution"));
        }

        let temp_dir = std::env::temp_dir();
        let repo_clone_path = temp_dir.join(format!("jarvis-clone-{}-{}", p.owner, p.repo));

        if !repo_clone_path.exists() {
            info!("cloning repository to {:?}", repo_clone_path);
            let clone_url = format!(
                "https://{}@github.com/{}/{}.git",
                p.github_pat, p.owner, p.repo
            );
            let output = Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    &clone_url,
                    repo_clone_path.to_str().unwrap(),
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("git clone failed: {}", stderr));
            }
        }

        let branch_name = plan.branch_name.clone();
        info!("creating branch {}", branch_name);

        let checkout_output = Command::new("git")
            .args(["checkout", "-b", &branch_name])
            .current_dir(&repo_clone_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !checkout_output.status.success() {
            let stderr = String::from_utf8_lossy(&checkout_output.stderr);
            if !stderr.contains("already exists") {
                warn!("git branch creation: {}", stderr);
            }
        }

        let config = ctx.config.as_ref().clone();
        let execution_result = ImplementationExecutor::execute(
            &config,
            session.auth_manager(),
            session.models_manager(),
            session.clone_session(),
            ctx.clone(),
            cancel_token.child_token(),
            &plan,
            &repo_context,
            &gh_client,
            &repo_clone_path,
        )
        .await;

        let execution = match execution_result {
            Ok(result) => {
                info!(
                    "execution complete: status={:?}, iterations={}",
                    result.status, result.iterations
                );
                Some(result)
            }
            Err(e) => {
                warn!("execution failed: {e}");
                Some(ExecutionResult {
                    status: ExecutionStatus::AgentError,
                    branch_name,
                    iterations: 0,
                    test_results: vec![],
                    agent_summary: None,
                    pr_url: None,
                    error: Some(e.to_string()),
                })
            }
        };

        Ok(IssueResolution {
            issue_number: p.issue_number,
            repo_full_name: format!("{}/{}", p.owner, p.repo),
            analysis,
            plan: Some(plan),
            safety: Some(safety),
            should_proceed: true,
            rejection_reason: None,
            execution,
        })
    }
}
