//! Core types for the issue resolver pipeline.
//!
//! These types flow through the analyzer, planner, and safety gate stages
//! of the autonomous issue-resolution workflow.

use serde::Deserialize;
use serde::Serialize;

use crate::safety::assessment::RiskLevel;

/// Complexity classification for an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueComplexity {
    /// Trivial change (typo fix, comment update, etc.)
    Trivial,
    /// Simple change touching 1-2 files with straightforward logic.
    Simple,
    /// Moderate change requiring multi-file edits and/or new tests.
    Moderate,
    /// Complex change requiring architectural understanding.
    Complex,
}

/// Category of the issue for routing and safety decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    BugFix,
    Feature,
    Refactor,
    Documentation,
    Test,
    Chore,
    Security,
}

/// LLM-produced analysis of a GitHub issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAnalysis {
    /// Short summary of what the issue is asking for.
    pub summary: String,
    /// Complexity classification.
    pub complexity: IssueComplexity,
    /// Category of the issue.
    pub category: IssueCategory,
    /// Files estimated to require changes.
    pub estimated_files: Vec<String>,
    /// High-level approach to resolve the issue.
    pub approach: String,
    /// Whether tests are needed for the change.
    pub tests_needed: bool,
    /// Identified risks.
    pub risks: Vec<String>,
    /// Whether the issue can be resolved autonomously.
    pub can_auto_resolve: bool,
    /// Reasoning for the auto-resolve decision.
    pub auto_resolve_reasoning: String,
    /// Confidence in the analysis (0.0 to 1.0).
    pub confidence: f32,
}

/// Contextual information about the target repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoContext {
    /// Repository owner.
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Primary programming language.
    pub language: Option<String>,
    /// Build system / framework detected.
    pub framework: Option<String>,
    /// Relevant files discovered during context gathering.
    pub relevant_files: Vec<RelevantFile>,
    /// Repository tree structure (truncated summary).
    pub tree_summary: String,
    /// README contents (if available).
    pub readme: Option<String>,
    /// Detected coding patterns and conventions.
    pub patterns: Vec<String>,
    /// Default branch name (e.g., "main").
    pub default_branch: String,
}

/// A file that is relevant to the issue being resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevantFile {
    /// Path relative to repo root.
    pub path: String,
    /// Why this file is relevant.
    pub reason: String,
    /// Decoded content (if fetched).
    pub content: Option<String>,
}

/// A single step in the implementation plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationStep {
    /// Step number (1-indexed).
    pub step_number: usize,
    /// Description of what to do.
    pub description: String,
    /// File to modify or create.
    pub file_path: String,
    /// Type of change: "modify", "create", "delete".
    pub change_type: String,
    /// Detailed instructions for the change.
    pub instructions: String,
    /// Dependencies on prior steps (step numbers).
    pub dependencies: Vec<usize>,
}

/// LLM-produced implementation plan for resolving an issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    /// The issue analysis this plan is based on.
    pub analysis: IssueAnalysis,
    /// Ordered list of implementation steps.
    pub steps: Vec<ImplementationStep>,
    /// Suggested branch name for the PR.
    pub branch_name: String,
    /// Suggested commit message.
    pub commit_message: String,
    /// Suggested PR title.
    pub pr_title: String,
    /// Suggested PR body.
    pub pr_body: String,
    /// Test commands to run after implementation.
    pub test_commands: Vec<String>,
    /// Overall confidence in the plan (0.0 to 1.0).
    pub confidence: f32,
}

/// Combined safety assessment for the issue resolution pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSafetyAssessment {
    /// Whether the issue is safe to resolve autonomously.
    pub is_safe: bool,
    /// Overall risk level.
    pub risk_level: RiskLevel,
    /// Reasoning for the assessment.
    pub reasoning: String,
    /// Whether a human should review the plan before execution.
    pub requires_human_review: bool,
    /// Per-step risk assessments (step_number -> risk_level).
    pub step_risks: Vec<StepRisk>,
    /// Confidence in the safety assessment (0.0 to 1.0).
    pub confidence: f32,
}

/// Risk assessment for a single implementation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRisk {
    pub step_number: usize,
    pub risk_level: RiskLevel,
    pub reasoning: String,
}

/// Status of the code implementation execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Implementation succeeded — code compiles, tests pass.
    Success,
    /// Implementation completed but tests are failing.
    TestsFailure,
    /// The sub-agent encountered an error or was unable to complete.
    AgentError,
    /// Execution was cancelled (e.g., timeout or user request).
    Cancelled,
    /// Maximum fix iterations exceeded with tests still failing.
    MaxIterationsExceeded,
}

/// Result of running a single test command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    /// The test command that was executed.
    pub command: String,
    /// Whether the test command succeeded (exit code 0).
    pub passed: bool,
    /// Combined stdout + stderr output (truncated).
    pub output: String,
}

/// Result of the code implementation executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Final status of the execution.
    pub status: ExecutionStatus,
    /// Name of the branch where changes were committed.
    pub branch_name: String,
    /// Number of fix→test iterations performed.
    pub iterations: u32,
    /// Results from the last test run (if any).
    pub test_results: Vec<TestRunResult>,
    /// The last agent message (summary of what was done).
    pub agent_summary: Option<String>,
    /// URL of the created pull request (set later in Phase 4).
    pub pr_url: Option<String>,
    /// Error message if the execution failed.
    pub error: Option<String>,
}

/// Result of the full issue resolver pipeline (analysis + plan + safety + execution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueResolution {
    /// The GitHub issue number.
    pub issue_number: u64,
    /// Repository owner/name.
    pub repo_full_name: String,
    /// The analysis produced by the analyzer.
    pub analysis: IssueAnalysis,
    /// The implementation plan (if analysis says auto-resolvable).
    pub plan: Option<ImplementationPlan>,
    /// Safety assessment of the plan.
    pub safety: Option<IssueSafetyAssessment>,
    /// Whether to proceed with autonomous implementation.
    pub should_proceed: bool,
    /// If not proceeding, the reason why.
    pub rejection_reason: Option<String>,
    /// Result of the code implementation (if execution was attempted).
    pub execution: Option<ExecutionResult>,
}
