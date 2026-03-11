//! Autonomous issue resolver pipeline.
//!
//! This module implements a multi-stage pipeline for automatically resolving
//! GitHub issues:
//!
//! 1. **Scanner** — polls repositories for issues with configured labels.
//! 2. **Context** — gathers repository structure, languages, and relevant files.
//! 3. **Analyzer** — uses an LLM sub-agent to produce a structured analysis.
//! 4. **Planner** — uses an LLM sub-agent to produce an implementation plan.
//! 5. **Safety gate** — evaluates the plan against the safety classifier.
//!
//! The entry-point is [`IssueResolverTask`], which implements
//! [`SessionTask`](crate::tasks::SessionTask) and orchestrates the full pipeline.

pub mod analyzer;
pub mod context;
pub mod executor;
pub mod planner;
pub mod safety_gate;
pub mod scanner;
pub mod task;
pub mod types;

pub use executor::ImplementationExecutor;
pub use scanner::IssueScanner;
pub use scanner::ScannerConfig;
pub use task::IssueResolverParams;
pub use task::IssueResolverTask;
pub use types::ExecutionResult;
pub use types::ExecutionStatus;
pub use types::ImplementationPlan;
pub use types::IssueAnalysis;
pub use types::IssueResolution;
pub use types::IssueSafetyAssessment;
pub use types::RepoContext;
pub use types::TestRunResult;
