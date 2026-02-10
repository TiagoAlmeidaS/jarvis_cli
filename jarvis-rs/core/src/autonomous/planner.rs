//! Execution planner for autonomous decision-making.

use crate::autonomous::context::AnalyzedContext;
use crate::capability::metadata::CapabilityMetadata;
use crate::capability::registry::CapabilityRegistry;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Represents an execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Steps in the execution plan
    pub steps: Vec<ExecutionStep>,
    /// Estimated total time
    pub estimated_time: String,
    /// Confidence in the plan (0.0 to 1.0)
    pub confidence: f32,
    /// Risks identified
    pub risks: Vec<String>,
}

/// A step in the execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step number
    pub step_number: usize,
    /// Capability to use
    pub capability_id: String,
    /// Capability name
    pub capability_name: String,
    /// Description of what this step does
    pub description: String,
    /// Parameters for the capability
    pub parameters: std::collections::HashMap<String, String>,
    /// Estimated time for this step
    pub estimated_time: String,
    /// Dependencies (other steps that must complete first)
    pub dependencies: Vec<usize>,
}

/// Trait for execution planning.
#[async_trait::async_trait]
pub trait ExecutionPlanner: Send + Sync {
    /// Creates an execution plan based on analyzed context.
    async fn create_plan(
        &self,
        context: &AnalyzedContext,
        registry: &dyn CapabilityRegistry,
    ) -> Result<ExecutionPlan>;
}

/// Rule-based execution planner.
///
/// This planner matches capabilities to context requirements.
/// In production, this would use more sophisticated matching algorithms.
pub struct RuleBasedExecutionPlanner;

impl RuleBasedExecutionPlanner {
    /// Creates a new rule-based execution planner.
    pub fn new() -> Self {
        Self
    }

    /// Matches capabilities to context requirements.
    async fn match_capabilities(
        &self,
        context: &AnalyzedContext,
        registry: &dyn CapabilityRegistry,
    ) -> Result<Vec<CapabilityMetadata>> {
        let mut matches = Vec::new();

        // Search for capabilities matching the intent
        let search_results = registry.search(&context.intent).await?;

        // Filter by availability and relevance
        for capability in search_results {
            if capability.is_available() {
                // Simple relevance scoring
                let relevance = self.calculate_relevance(&capability, context);
                if relevance > 0.3 {
                    matches.push(capability);
                }
            }
        }

        // Sort by relevance (in production, would use actual relevance scores)
        matches.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        Ok(matches)
    }

    /// Calculates relevance score for a capability.
    fn calculate_relevance(&self, capability: &CapabilityMetadata, context: &AnalyzedContext) -> f32 {
        let mut score: f32 = 0.0;

        // Check if capability name/description matches intent
        let intent_lower = context.intent.to_lowercase();
        if capability.name.to_lowercase().contains(&intent_lower)
            || capability.description.to_lowercase().contains(&intent_lower)
        {
            score += 0.5;
        }

        // Check if capability tags match entities
        for entity in &context.entities {
            if capability.tags.iter().any(|tag| tag.to_lowercase().contains(&entity.value.to_lowercase())) {
                score += 0.3;
            }
        }

        // Boost score based on usage count (popular = more reliable)
        if capability.usage_count > 0 {
            score += 0.2;
        }

        score.min(1.0)
    }

    /// Creates execution steps from matched capabilities.
    fn create_steps(&self, capabilities: &[CapabilityMetadata]) -> Vec<ExecutionStep> {
        capabilities
            .iter()
            .enumerate()
            .map(|(idx, cap)| ExecutionStep {
                step_number: idx + 1,
                capability_id: cap.id.clone(),
                capability_name: cap.name.clone(),
                description: cap.description.clone(),
                parameters: std::collections::HashMap::new(),
                estimated_time: self.estimate_step_time(cap),
                dependencies: if idx > 0 { vec![idx] } else { vec![] },
            })
            .collect()
    }

    /// Estimates time for a step based on capability metadata.
    fn estimate_step_time(&self, capability: &CapabilityMetadata) -> String {
        if let Some(avg_time) = capability.performance_metadata.avg_execution_time_ms {
            format!("{}ms", avg_time)
        } else {
            match capability.performance_metadata.performance_profile {
                crate::capability::metadata::PerformanceProfile::Fast => "100ms".to_string(),
                crate::capability::metadata::PerformanceProfile::Medium => "500ms".to_string(),
                crate::capability::metadata::PerformanceProfile::Slow => "2s".to_string(),
            }
        }
    }
}

impl Default for RuleBasedExecutionPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ExecutionPlanner for RuleBasedExecutionPlanner {
    async fn create_plan(
        &self,
        context: &AnalyzedContext,
        registry: &dyn CapabilityRegistry,
    ) -> Result<ExecutionPlan> {
        // Match capabilities to context
        let matched_capabilities = self.match_capabilities(context, registry).await?;

        if matched_capabilities.is_empty() {
            return Err(anyhow::anyhow!("No matching capabilities found"));
        }

        // Create execution steps
        let steps = self.create_steps(&matched_capabilities);

        // Calculate total estimated time
        let total_time_ms: u64 = steps
            .iter()
            .filter_map(|s| {
                s.estimated_time
                    .trim_end_matches("ms")
                    .trim_end_matches('s')
                    .parse::<u64>()
                    .ok()
            })
            .sum();

        let estimated_time = if total_time_ms < 1000 {
            format!("{}ms", total_time_ms)
        } else {
            format!("{:.1}s", total_time_ms as f64 / 1000.0)
        };

        // Identify risks
        let risks = vec![
            "Capability execution may fail".to_string(),
            "Dependencies may not be available".to_string(),
        ];

        Ok(ExecutionPlan {
            steps,
            estimated_time,
            confidence: context.confidence,
            risks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autonomous::context::{AnalyzedContext, ContextAnalyzer, RuleBasedContextAnalyzer};
    use crate::capability::metadata::{CapabilityMetadata, CapabilityType};
    use crate::capability::registry::InMemoryCapabilityRegistry;

    #[tokio::test]
    async fn test_create_plan() {
        let planner = RuleBasedExecutionPlanner::new();
        let registry = InMemoryCapabilityRegistry::new();
        
        // Register a test capability
        let capability = CapabilityMetadata::new(
            "test-id".to_string(),
            "test-capability".to_string(),
            CapabilityType::Tool,
            "A test capability for REST API".to_string(),
        );
        registry.register(capability).await.unwrap();

        // Analyze context
        let analyzer = RuleBasedContextAnalyzer::new();
        let state = std::collections::HashMap::new();
        let context = analyzer
            .analyze("Create REST API", &state)
            .await
            .unwrap();

        // Create plan
        let plan = planner.create_plan(&context, &registry).await.unwrap();

        assert!(!plan.steps.is_empty());
        assert!(plan.confidence > 0.0);
    }
}
