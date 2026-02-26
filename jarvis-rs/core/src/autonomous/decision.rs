//! Autonomous decision engine for making execution decisions.

use crate::autonomous::context::AnalyzedContext;
use crate::autonomous::context::ContextAnalyzer;
use crate::autonomous::planner::ExecutionPlan;
use crate::autonomous::planner::ExecutionPlanner;
use crate::capability::registry::CapabilityRegistry;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

/// Represents a decision made by the decision engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Whether to proceed with execution
    pub should_execute: bool,
    /// The execution plan if proceeding
    pub plan: Option<ExecutionPlan>,
    /// Reasoning for the decision
    pub reasoning: String,
    /// Confidence in the decision (0.0 to 1.0)
    pub confidence: f32,
    /// Alternative approaches considered
    pub alternatives: Vec<String>,
}

/// Trait for autonomous decision engine.
#[async_trait::async_trait]
pub trait AutonomousDecisionEngine: Send + Sync {
    /// Makes a decision based on context and available capabilities.
    async fn make_decision(
        &self,
        context: &AnalyzedContext,
        registry: &dyn CapabilityRegistry,
    ) -> Result<Decision>;
}

/// Rule-based autonomous decision engine.
///
/// This engine uses rules and heuristics to make decisions.
/// In production, this would integrate with LLM for more sophisticated reasoning.
pub struct RuleBasedDecisionEngine {
    /// Context analyzer
    context_analyzer: Box<dyn ContextAnalyzer>,
    /// Execution planner
    planner: Box<dyn ExecutionPlanner>,
    /// Minimum confidence threshold for execution
    min_confidence_threshold: f32,
}

impl RuleBasedDecisionEngine {
    /// Creates a new rule-based decision engine.
    pub fn new(
        context_analyzer: Box<dyn ContextAnalyzer>,
        planner: Box<dyn ExecutionPlanner>,
        min_confidence_threshold: f32,
    ) -> Self {
        Self {
            context_analyzer,
            planner,
            min_confidence_threshold: min_confidence_threshold.max(0.0).min(1.0),
        }
    }

    /// Determines if execution should proceed.
    fn should_execute(&self, context: &AnalyzedContext, plan: &ExecutionPlan) -> bool {
        // Check confidence thresholds
        if context.confidence < self.min_confidence_threshold {
            return false;
        }

        if plan.confidence < self.min_confidence_threshold {
            return false;
        }

        // Check if plan has steps
        if plan.steps.is_empty() {
            return false;
        }

        // Check for critical risks
        let critical_risks = plan.risks.iter().any(|r| {
            r.to_lowercase().contains("critical") || r.to_lowercase().contains("critical")
        });

        !critical_risks
    }

    /// Generates reasoning for the decision.
    fn generate_reasoning(
        &self,
        context: &AnalyzedContext,
        plan: &ExecutionPlan,
        should_execute: bool,
    ) -> String {
        if should_execute {
            format!(
                "Context analyzed with {:.0}% confidence. Plan created with {} steps. Estimated time: {}. Proceeding with execution.",
                context.confidence * 100.0,
                plan.steps.len(),
                plan.estimated_time
            )
        } else {
            format!(
                "Context confidence ({:.0}%) or plan confidence ({:.0}%) below threshold ({:.0}%). Cannot proceed safely.",
                context.confidence * 100.0,
                plan.confidence * 100.0,
                self.min_confidence_threshold * 100.0
            )
        }
    }

    /// Generates alternative approaches.
    fn generate_alternatives(&self, context: &AnalyzedContext) -> Vec<String> {
        let mut alternatives = Vec::new();

        // Suggest manual approach if confidence is low
        if context.confidence < 0.6 {
            alternatives.push("Consider manual implementation for better control".to_string());
        }

        // Suggest breaking down into smaller steps
        if context.requirements.len() > 3 {
            alternatives.push("Consider breaking down into smaller, incremental steps".to_string());
        }

        alternatives
    }
}

impl Default for RuleBasedDecisionEngine {
    fn default() -> Self {
        Self::new(
            Box::new(crate::autonomous::context::RuleBasedContextAnalyzer::default()),
            Box::new(crate::autonomous::planner::RuleBasedExecutionPlanner::default()),
            0.6,
        )
    }
}

#[async_trait::async_trait]
impl AutonomousDecisionEngine for RuleBasedDecisionEngine {
    async fn make_decision(
        &self,
        context: &AnalyzedContext,
        registry: &dyn CapabilityRegistry,
    ) -> Result<Decision> {
        // Create execution plan
        let plan = self.planner.create_plan(context, registry).await?;

        // Determine if should execute
        let should_execute = self.should_execute(context, &plan);

        // Generate reasoning
        let reasoning = self.generate_reasoning(context, &plan, should_execute);

        // Generate alternatives
        let alternatives = self.generate_alternatives(context);

        // Calculate overall confidence
        let confidence = (context.confidence + plan.confidence) / 2.0;

        Ok(Decision {
            should_execute,
            plan: if should_execute { Some(plan) } else { None },
            reasoning,
            confidence,
            alternatives,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autonomous::context::AnalyzedContext;
    use crate::autonomous::context::ContextAnalyzer;
    use crate::autonomous::context::RuleBasedContextAnalyzer;
    use crate::capability::metadata::CapabilityMetadata;
    use crate::capability::metadata::CapabilityType;
    use crate::capability::registry::InMemoryCapabilityRegistry;

    #[tokio::test]
    async fn test_make_decision() {
        let engine = RuleBasedDecisionEngine::default();
        let registry = InMemoryCapabilityRegistry::new();

        // Register capability
        let capability = CapabilityMetadata::new(
            "test-id".to_string(),
            "test-capability".to_string(),
            CapabilityType::Tool,
            "Test capability".to_string(),
        );
        registry.register(capability).await.unwrap();

        // Create context
        let analyzer = RuleBasedContextAnalyzer::new();
        let state = std::collections::HashMap::new();
        let context = analyzer.analyze("Create REST API", &state).await.unwrap();

        // Make decision
        let decision = engine.make_decision(&context, &registry).await.unwrap();

        assert!(!decision.reasoning.is_empty());
        assert!(decision.confidence > 0.0);
    }
}
