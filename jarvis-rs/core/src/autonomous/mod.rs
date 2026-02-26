//! Autonomous decision-making system.
//!
//! This module provides components for autonomous decision-making, including
//! context analysis, execution planning, and decision engines.

pub mod context;
pub mod decision;
pub mod planner;

pub use context::AnalyzedContext;
pub use context::ContextAnalysisError;
pub use context::ContextAnalyzer;
pub use context::RuleBasedContextAnalyzer;
pub use decision::AutonomousDecisionEngine;
pub use decision::Decision;
pub use decision::RuleBasedDecisionEngine;
pub use planner::ExecutionPlan;
pub use planner::ExecutionPlanner;
pub use planner::ExecutionStep;
pub use planner::RuleBasedExecutionPlanner;
