// Analytics module for self-improvement and metrics
//
// This module provides tools for Jarvis to analyze its own performance,
// identify areas for improvement, and provide actionable insights.

mod metrics;
mod queries;
mod self_improvement;
mod suggestions;

pub use metrics::CacheMetrics;
pub use metrics::CommandMetrics;
pub use metrics::PerformanceMetrics;
pub use metrics::SkillMetrics;
pub use metrics::SystemMetrics;
pub use queries::AnalyticsQueries;
pub use self_improvement::SelfImprovement;
pub use suggestions::Improvement;
pub use suggestions::ImprovementCategory;
pub use suggestions::ImprovementPriority;
pub use suggestions::group_by_priority;
