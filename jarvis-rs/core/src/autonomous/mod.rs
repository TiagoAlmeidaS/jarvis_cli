//! Autonomous decision-making system.
//!
//! This module provides components for autonomous decision-making, including
//! context analysis, execution planning, and decision engines.

pub mod context;
pub mod decision;
pub mod planner;

pub use context::{AnalyzedContext, ContextAnalyzer, ContextAnalysisError, RuleBasedContextAnalyzer};
pub use decision::{AutonomousDecisionEngine, Decision, RuleBasedDecisionEngine};
pub use planner::{ExecutionPlan, ExecutionPlanner, ExecutionStep, RuleBasedExecutionPlanner};
