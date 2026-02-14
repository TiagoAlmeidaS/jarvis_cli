// Analytics module for self-improvement and metrics
//
// This module provides tools for Jarvis to analyze its own performance,
// identify areas for improvement, and provide actionable insights.

mod metrics;
mod queries;
mod self_improvement;
mod suggestions;

pub use metrics::{CacheMetrics, CommandMetrics, PerformanceMetrics, SkillMetrics, SystemMetrics};
pub use queries::AnalyticsQueries;
pub use self_improvement::SelfImprovement;
pub use suggestions::{Improvement, ImprovementCategory, ImprovementPriority, group_by_priority};
