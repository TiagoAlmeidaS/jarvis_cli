//! Safety gate for the issue resolver pipeline.
//!
//! Integrates with the existing [`SafetyClassifier`](crate::safety::classifier::SafetyClassifier)
//! to assess each step in an [`ImplementationPlan`] and produce an aggregate
//! [`IssueSafetyAssessment`].

use anyhow::Result;

use crate::safety::assessment::RiskLevel;
use crate::safety::classifier::ProposedAction;
use crate::safety::classifier::SafetyClassifier;

use super::types::ImplementationPlan;
use super::types::IssueCategory;
use super::types::IssueComplexity;
use super::types::IssueSafetyAssessment;
use super::types::StepRisk;

/// Assess the safety of an entire implementation plan.
pub async fn assess_plan(
    classifier: &dyn SafetyClassifier,
    plan: &ImplementationPlan,
) -> Result<IssueSafetyAssessment> {
    let mut step_risks = Vec::new();
    let mut highest_risk = RiskLevel::Low;

    for step in &plan.steps {
        let category = categorize_change(&step.change_type, &step.file_path);

        let action = ProposedAction {
            action_type: format!("issue_resolver_{}", step.change_type),
            files: vec![step.file_path.clone()],
            change: step.description.clone(),
            impact: step.instructions.clone(),
            category: Some(category),
        };

        let assessment = classifier.assess_action(&action).await?;

        if assessment.risk_level > highest_risk {
            highest_risk = assessment.risk_level.clone();
        }

        step_risks.push(StepRisk {
            step_number: step.step_number,
            risk_level: assessment.risk_level,
            reasoning: assessment.reasoning,
        });
    }

    // Apply additional heuristics based on the issue analysis.
    let analysis_risk = analysis_level_risk(&plan.analysis.complexity, &plan.analysis.category);
    if analysis_risk > highest_risk {
        highest_risk = analysis_risk;
    }

    let is_safe = matches!(highest_risk, RiskLevel::Low);
    let requires_human_review = matches!(highest_risk, RiskLevel::High | RiskLevel::Critical);

    let reasoning = if is_safe {
        "All steps assessed as low risk. Safe for autonomous execution.".to_string()
    } else if requires_human_review {
        format!("Overall risk level is {highest_risk:?}. Human review required before execution.")
    } else {
        format!("Overall risk level is {highest_risk:?}. Proceeding with caution.")
    };

    // Confidence is the average of the plan confidence and inverse of risk.
    let risk_penalty = match highest_risk {
        RiskLevel::Low => 0.0,
        RiskLevel::Medium => 0.15,
        RiskLevel::High => 0.3,
        RiskLevel::Critical => 0.5,
    };
    let confidence = (plan.confidence - risk_penalty).max(0.1);

    Ok(IssueSafetyAssessment {
        is_safe,
        risk_level: highest_risk,
        reasoning,
        requires_human_review,
        step_risks,
        confidence,
    })
}

/// Map a change type + file path to a safety category string
/// understood by the existing `RuleBasedSafetyClassifier`.
fn categorize_change(change_type: &str, file_path: &str) -> String {
    let path_lower = file_path.to_lowercase();

    if path_lower.contains("test") || path_lower.contains("spec") {
        return "test_file".to_string();
    }
    if path_lower.ends_with(".md") || path_lower.contains("doc") {
        return "documentation".to_string();
    }
    if change_type == "delete" {
        return "production_code".to_string();
    }
    if path_lower.contains("config")
        || path_lower.ends_with(".toml")
        || path_lower.ends_with(".yml")
        || path_lower.ends_with(".yaml")
    {
        return "config_file".to_string();
    }
    if path_lower.contains("migration") || path_lower.contains("schema") {
        return "database".to_string();
    }
    if path_lower.contains("auth")
        || path_lower.contains("security")
        || path_lower.contains("secret")
    {
        return "security".to_string();
    }

    "production_code".to_string()
}

/// Determine a baseline risk level from the issue's complexity and category.
fn analysis_level_risk(complexity: &IssueComplexity, category: &IssueCategory) -> RiskLevel {
    match category {
        IssueCategory::Security => RiskLevel::Critical,
        IssueCategory::Documentation => RiskLevel::Low,
        IssueCategory::Test => RiskLevel::Low,
        IssueCategory::Chore => RiskLevel::Low,
        _ => match complexity {
            IssueComplexity::Trivial => RiskLevel::Low,
            IssueComplexity::Simple => RiskLevel::Medium,
            IssueComplexity::Moderate => RiskLevel::High,
            IssueComplexity::Complex => RiskLevel::Critical,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_change_test_file() {
        assert_eq!(
            categorize_change("modify", "tests/test_main.rs"),
            "test_file"
        );
    }

    #[test]
    fn test_categorize_change_documentation() {
        assert_eq!(categorize_change("modify", "README.md"), "documentation");
    }

    #[test]
    fn test_categorize_change_config() {
        assert_eq!(
            categorize_change("modify", "config/settings.toml"),
            "config_file"
        );
    }

    #[test]
    fn test_categorize_change_security() {
        assert_eq!(
            categorize_change("modify", "src/auth/handler.rs"),
            "security"
        );
    }

    #[test]
    fn test_categorize_change_production() {
        assert_eq!(categorize_change("modify", "src/lib.rs"), "production_code");
    }

    #[test]
    fn test_analysis_level_risk_security() {
        assert_eq!(
            analysis_level_risk(&IssueComplexity::Simple, &IssueCategory::Security),
            RiskLevel::Critical
        );
    }

    #[test]
    fn test_analysis_level_risk_documentation() {
        assert_eq!(
            analysis_level_risk(&IssueComplexity::Simple, &IssueCategory::Documentation),
            RiskLevel::Low
        );
    }

    #[test]
    fn test_analysis_level_risk_complex_feature() {
        assert_eq!(
            analysis_level_risk(&IssueComplexity::Complex, &IssueCategory::Feature),
            RiskLevel::Critical
        );
    }

    #[test]
    fn test_analysis_level_risk_trivial_bugfix() {
        assert_eq!(
            analysis_level_risk(&IssueComplexity::Trivial, &IssueCategory::BugFix),
            RiskLevel::Low
        );
    }

    #[tokio::test]
    async fn test_assess_plan_with_test_files_only() {
        use crate::issue_resolver::types::*;
        use crate::safety::classifier::RuleBasedSafetyClassifier;

        let classifier = RuleBasedSafetyClassifier::default();

        let plan = ImplementationPlan {
            analysis: IssueAnalysis {
                summary: "Fix test".to_string(),
                complexity: IssueComplexity::Trivial,
                category: IssueCategory::Test,
                estimated_files: vec!["tests/test.rs".to_string()],
                approach: "Fix assertion".to_string(),
                tests_needed: false,
                risks: vec![],
                can_auto_resolve: true,
                auto_resolve_reasoning: "Test fix".to_string(),
                confidence: 0.95,
            },
            steps: vec![ImplementationStep {
                step_number: 1,
                description: "Fix test assertion".to_string(),
                file_path: "tests/test.rs".to_string(),
                change_type: "modify".to_string(),
                instructions: "Change assert".to_string(),
                dependencies: vec![],
            }],
            branch_name: "fix/test".to_string(),
            commit_message: "fix: test".to_string(),
            pr_title: "Fix test".to_string(),
            pr_body: "Fixes test".to_string(),
            test_commands: vec![],
            confidence: 0.95,
        };

        let assessment = assess_plan(&classifier, &plan).await.unwrap();
        assert!(assessment.is_safe);
        assert_eq!(assessment.risk_level, RiskLevel::Low);
        assert!(!assessment.requires_human_review);
    }
}
