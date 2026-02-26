//! Plan agent for creating detailed implementation plans.

use crate::agent::session::AgentSession;
use crate::agent::session::AgentSessionManager;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Result of a plan agent operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAgentResult {
    /// The generated plan in markdown format
    pub plan: String,
    /// Implementation steps
    pub steps: Vec<ImplementationStep>,
    /// Analysis section
    pub analysis: String,
    /// Trade-offs considered
    pub trade_offs: Vec<TradeOff>,
    /// Critical files identified
    pub critical_files: Vec<String>,
    /// Risks identified
    pub risks: Vec<Risk>,
    /// Estimated time/complexity
    pub estimates: PlanEstimates,
    /// Whether planning completed successfully
    pub success: bool,
}

/// An implementation step in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationStep {
    /// Step number
    pub step_number: usize,
    /// Step description
    pub description: String,
    /// Files to modify/create
    pub files: Vec<String>,
    /// Dependencies required
    pub dependencies: Vec<String>,
    /// Estimated time
    pub estimated_time: String,
}

/// A trade-off analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOff {
    /// Approach name
    pub approach: String,
    /// Pros
    pub pros: Vec<String>,
    /// Cons
    pub cons: Vec<String>,
    /// Recommendation
    pub recommendation: Option<String>,
}

/// A risk identified in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    /// Risk level (low, medium, high)
    pub level: String,
    /// Risk description
    pub description: String,
    /// Mitigation strategy
    pub mitigation: Option<String>,
}

/// Time and complexity estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEstimates {
    /// Total estimated time
    pub total_time: String,
    /// Complexity level (low, medium, high)
    pub complexity: String,
    /// Number of steps
    pub step_count: usize,
}

/// Trait for plan agent functionality.
#[async_trait::async_trait]
pub trait PlanAgent: Send + Sync {
    /// Creates an implementation plan based on requirements.
    async fn create_plan(
        &self,
        requirements: &str,
        session: &mut AgentSession,
    ) -> Result<PlanAgentResult>;
}

/// Rule-based plan agent implementation.
///
/// This agent creates implementation plans using structured analysis.
/// In production, this would integrate with LLM and exploration tools.
pub struct RuleBasedPlanAgent {
    /// Session manager for maintaining context
    session_manager: Arc<dyn AgentSessionManager>,
}

impl RuleBasedPlanAgent {
    /// Creates a new plan agent.
    pub fn new(session_manager: Arc<dyn AgentSessionManager>) -> Self {
        Self { session_manager }
    }

    /// Generates analysis section.
    fn generate_analysis(&self, requirements: &str) -> String {
        format!(
            r#"## Analysis

The requirements specify: **{}**

### Key Requirements:
- Functional requirements need to be identified
- Technical constraints should be considered
- Integration points must be analyzed

### Architecture Considerations:
- Code structure and organization
- Dependencies and external libraries
- Testing strategy
- Error handling approach
"#,
            requirements
        )
    }

    /// Generates trade-offs section.
    fn generate_trade_offs(&self) -> Vec<TradeOff> {
        vec![
            TradeOff {
                approach: "Approach A".to_string(),
                pros: vec![
                    "Simple implementation".to_string(),
                    "Fast to develop".to_string(),
                ],
                cons: vec!["May not scale well".to_string()],
                recommendation: Some("Use for MVP".to_string()),
            },
            TradeOff {
                approach: "Approach B".to_string(),
                pros: vec!["Scalable".to_string(), "Maintainable".to_string()],
                cons: vec![
                    "More complex".to_string(),
                    "Longer development time".to_string(),
                ],
                recommendation: Some("Use for production".to_string()),
            },
        ]
    }

    /// Generates implementation steps.
    fn generate_steps(&self, requirements: &str) -> Vec<ImplementationStep> {
        vec![
            ImplementationStep {
                step_number: 1,
                description: "Set up project structure and dependencies".to_string(),
                files: vec!["Cargo.toml".to_string(), "src/lib.rs".to_string()],
                dependencies: vec![],
                estimated_time: "30 minutes".to_string(),
            },
            ImplementationStep {
                step_number: 2,
                description: "Implement core functionality".to_string(),
                files: vec!["src/core.rs".to_string()],
                dependencies: vec![],
                estimated_time: "2 hours".to_string(),
            },
            ImplementationStep {
                step_number: 3,
                description: "Add tests and documentation".to_string(),
                files: vec![
                    "tests/integration_test.rs".to_string(),
                    "README.md".to_string(),
                ],
                dependencies: vec![],
                estimated_time: "1 hour".to_string(),
            },
        ]
    }

    /// Generates risks section.
    fn generate_risks(&self) -> Vec<Risk> {
        vec![
            Risk {
                level: "medium".to_string(),
                description: "Integration complexity with existing codebase".to_string(),
                mitigation: Some("Thorough testing and incremental integration".to_string()),
            },
            Risk {
                level: "low".to_string(),
                description: "Performance considerations".to_string(),
                mitigation: Some("Profile and optimize as needed".to_string()),
            },
        ]
    }

    /// Generates the full plan markdown.
    fn generate_plan_markdown(
        &self,
        requirements: &str,
        analysis: &str,
        steps: &[ImplementationStep],
        trade_offs: &[TradeOff],
        risks: &[Risk],
        critical_files: &[String],
        estimates: &PlanEstimates,
    ) -> String {
        let steps_md = steps
            .iter()
            .map(|step| {
                format!(
                    r#"### Step {}: {}

**Files**: {}
**Estimated Time**: {}

"#,
                    step.step_number,
                    step.description,
                    step.files.join(", "),
                    step.estimated_time
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let trade_offs_md = trade_offs
            .iter()
            .map(|to| {
                let pros = to
                    .pros
                    .iter()
                    .map(|p| format!("- {}", p))
                    .collect::<Vec<_>>()
                    .join("\n");
                let cons = to
                    .cons
                    .iter()
                    .map(|c| format!("- {}", c))
                    .collect::<Vec<_>>()
                    .join("\n");
                let rec = to
                    .recommendation
                    .as_ref()
                    .map(|r| format!("\n**Recommendation**: {}", r))
                    .unwrap_or_default();
                format!(
                    r#"#### {}

**Pros:**
{}

**Cons:**
{}{}
"#,
                    to.approach, pros, cons, rec
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let risks_md = risks
            .iter()
            .map(|risk| {
                let mitigation = risk
                    .mitigation
                    .as_ref()
                    .map(|m| format!("\n**Mitigation**: {}", m))
                    .unwrap_or_default();
                format!(
                    r#"### {} Risk: {}{}
"#,
                    risk.level, risk.description, mitigation
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"# Implementation Plan

## Requirements
{}

{}

## Trade-offs

{}

## Implementation Steps

{}

## Critical Files

{}

## Risks

{}

## Estimates

- **Total Time**: {}
- **Complexity**: {}
- **Steps**: {}
"#,
            requirements,
            analysis,
            trade_offs_md,
            steps_md,
            critical_files.join("\n"),
            risks_md,
            estimates.total_time,
            estimates.complexity,
            estimates.step_count
        )
    }
}

#[async_trait::async_trait]
impl PlanAgent for RuleBasedPlanAgent {
    async fn create_plan(
        &self,
        requirements: &str,
        session: &mut AgentSession,
    ) -> Result<PlanAgentResult> {
        // Add requirements to session
        self.session_manager
            .add_message(&session.session_id, "user", requirements)
            .await
            .map_err(|e| anyhow::anyhow!("Session error: {}", e))?;

        // Generate plan components
        let analysis = self.generate_analysis(requirements);
        let trade_offs = self.generate_trade_offs();
        let steps = self.generate_steps(requirements);
        let risks = self.generate_risks();
        let critical_files = steps
            .iter()
            .flat_map(|s| s.files.clone())
            .collect::<Vec<_>>();

        let estimates = PlanEstimates {
            total_time: "3.5 hours".to_string(),
            complexity: "medium".to_string(),
            step_count: steps.len(),
        };

        let plan = self.generate_plan_markdown(
            requirements,
            &analysis,
            &steps,
            &trade_offs,
            &risks,
            &critical_files,
            &estimates,
        );

        // Update session with plan
        self.session_manager
            .add_message(&session.session_id, "assistant", &plan)
            .await
            .map_err(|e| anyhow::anyhow!("Session error: {}", e))?;

        session.context.current_task = Some(format!("Plan: {}", requirements));
        session
            .context
            .progress
            .insert("steps".to_string(), steps.len().to_string());

        Ok(PlanAgentResult {
            plan,
            steps,
            analysis,
            trade_offs,
            critical_files,
            risks,
            estimates,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::session::InMemoryAgentSessionManager;

    #[tokio::test]
    async fn test_create_plan() {
        let session_manager = Arc::new(InMemoryAgentSessionManager::new());
        let mut session = session_manager.create_session("plan").await.unwrap();
        let agent = RuleBasedPlanAgent::new(session_manager);

        let result = agent
            .create_plan("Create a REST API for managing products", &mut session)
            .await
            .unwrap();

        assert!(result.success);
        assert!(!result.plan.is_empty());
        assert!(!result.steps.is_empty());
        assert!(!result.analysis.is_empty());
    }

    #[tokio::test]
    async fn test_plan_contains_required_sections() {
        let session_manager = Arc::new(InMemoryAgentSessionManager::new());
        let mut session = session_manager.create_session("plan").await.unwrap();
        let agent = RuleBasedPlanAgent::new(session_manager);

        let result = agent
            .create_plan("Test requirements", &mut session)
            .await
            .unwrap();

        assert!(result.plan.contains("Implementation Plan"));
        assert!(result.plan.contains("Requirements"));
        assert!(result.plan.contains("Implementation Steps"));
        assert!(result.plan.contains("Risks"));
    }
}
