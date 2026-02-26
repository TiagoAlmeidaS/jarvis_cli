//! Safety classifier for assessing action safety.

use crate::safety::assessment::RiskLevel;
use crate::safety::assessment::SafetyAssessment;
use crate::safety::rules::SafetyRules;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

/// Represents a proposed action to be assessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    /// Type of action (e.g., "code_change", "file_operation")
    pub action_type: String,
    /// Files affected by the action
    pub files: Vec<String>,
    /// Description of the change
    pub change: String,
    /// Impact assessment
    pub impact: String,
    /// Category of the action (test_file, production_code, etc.)
    pub category: Option<String>,
}

/// Trait for safety classification.
#[async_trait::async_trait]
pub trait SafetyClassifier: Send + Sync {
    /// Assesses if an action is safe to execute autonomously.
    async fn assess_action(&self, action: &ProposedAction) -> Result<SafetyAssessment>;
}

/// Rule-based safety classifier.
///
/// This classifier uses rules and heuristics to assess action safety.
/// In production, this would integrate with LLM for more sophisticated analysis.
pub struct RuleBasedSafetyClassifier {
    /// Safety rules configuration
    rules: SafetyRules,
}

impl RuleBasedSafetyClassifier {
    /// Creates a new rule-based safety classifier.
    pub fn new(rules: SafetyRules) -> Self {
        Self { rules }
    }

    /// Determines risk level based on action characteristics.
    fn determine_risk_level(&self, action: &ProposedAction) -> RiskLevel {
        // Check if action is prohibited
        if self.rules.is_prohibited(&action.action_type) {
            return RiskLevel::Critical;
        }

        // Check if action is whitelisted
        if self.rules.is_whitelisted(&action.action_type) {
            return RiskLevel::Low;
        }

        // Determine risk based on category
        if let Some(category) = &action.category {
            match category.as_str() {
                "test_file" => RiskLevel::Low,
                "comment" => RiskLevel::Low,
                "documentation" => RiskLevel::Low,
                "production_code" => RiskLevel::High,
                "database" => RiskLevel::Critical,
                "config_file" => RiskLevel::High,
                "security" => RiskLevel::Critical,
                _ => RiskLevel::Medium,
            }
        } else {
            // Default to medium if category unknown
            RiskLevel::Medium
        }
    }

    /// Checks if action affects production code.
    fn affects_production_code(&self, files: &[String]) -> bool {
        files.iter().any(|f| {
            !f.contains("test")
                && !f.contains("spec")
                && !f.contains("_test")
                && (f.ends_with(".rs") || f.ends_with(".py") || f.ends_with(".js"))
        })
    }

    /// Checks if action affects test files only.
    fn affects_test_files_only(&self, files: &[String]) -> bool {
        !files.is_empty()
            && files
                .iter()
                .all(|f| f.contains("test") || f.contains("spec") || f.contains("_test"))
    }

    /// Generates reasoning for the assessment.
    fn generate_reasoning(&self, action: &ProposedAction, risk_level: &RiskLevel) -> String {
        match risk_level {
            RiskLevel::Low => {
                if self.affects_test_files_only(&action.files) {
                    "Action affects test files only, which don't affect production".to_string()
                } else if action.action_type.contains("comment")
                    || action.action_type.contains("doc")
                {
                    "Action affects comments/documentation only".to_string()
                } else {
                    "Action is whitelisted as safe for autonomous execution".to_string()
                }
            }
            RiskLevel::Medium => {
                "Action requires consideration. Review recommended before execution".to_string()
            }
            RiskLevel::High => {
                if self.affects_production_code(&action.files) {
                    "Action affects production code. Human approval required".to_string()
                } else {
                    "Action has high risk characteristics".to_string()
                }
            }
            RiskLevel::Critical => {
                if self.rules.is_prohibited(&action.action_type) {
                    format!(
                        "Action '{}' is prohibited from autonomous execution",
                        action.action_type
                    )
                } else {
                    "Action has critical risk. Must not execute autonomously".to_string()
                }
            }
        }
    }

    /// Performs safety checks.
    fn perform_safety_checks(&self, action: &ProposedAction) -> Vec<String> {
        let mut checks = Vec::new();

        // Check if action is prohibited
        if self.rules.is_prohibited(&action.action_type) {
            checks.push("Action is in prohibited list".to_string());
        }

        // Check if action is whitelisted
        if self.rules.is_whitelisted(&action.action_type) {
            checks.push("Action is in autonomous whitelist".to_string());
        }

        // Check file types
        if self.affects_test_files_only(&action.files) {
            checks.push("Only test files affected".to_string());
        }

        if self.affects_production_code(&action.files) {
            checks.push("Production code affected - requires approval".to_string());
        }

        // Check for destructive operations
        if action.change.to_lowercase().contains("delete")
            || action.change.to_lowercase().contains("remove")
        {
            checks.push("Destructive operation detected".to_string());
        }

        checks
    }
}

impl Default for RuleBasedSafetyClassifier {
    fn default() -> Self {
        Self::new(SafetyRules::default())
    }
}

#[async_trait::async_trait]
impl SafetyClassifier for RuleBasedSafetyClassifier {
    async fn assess_action(&self, action: &ProposedAction) -> Result<SafetyAssessment> {
        // Determine risk level
        let risk_level = self.determine_risk_level(action);

        // Perform safety checks
        let safety_checks = self.perform_safety_checks(action);

        // Generate reasoning
        let reasoning = self.generate_reasoning(action, &risk_level);

        // Determine if safe to execute
        let is_safe = matches!(risk_level, RiskLevel::Low);
        let requires_approval = !is_safe;

        // Calculate confidence
        let confidence = if self.rules.is_whitelisted(&action.action_type)
            || self.rules.is_prohibited(&action.action_type)
        {
            0.95
        } else {
            0.8
        };

        Ok(SafetyAssessment {
            is_safe_to_execute_autonomously: is_safe,
            risk_level,
            reasoning,
            requires_human_approval: requires_approval,
            safety_checks,
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safe_action_assessment() {
        let classifier = RuleBasedSafetyClassifier::default();
        let action = ProposedAction {
            action_type: "fix_test_file".to_string(),
            files: vec!["tests/test.rs".to_string()],
            change: "Fix test assertion".to_string(),
            impact: "Test file only".to_string(),
            category: Some("test_file".to_string()),
        };

        let assessment = classifier.assess_action(&action).await.unwrap();
        assert!(assessment.is_safe_to_execute_autonomously);
        assert_eq!(assessment.risk_level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn test_unsafe_action_assessment() {
        let classifier = RuleBasedSafetyClassifier::default();
        let action = ProposedAction {
            action_type: "delete_file".to_string(),
            files: vec!["src/main.rs".to_string()],
            change: "Delete file".to_string(),
            impact: "File deletion".to_string(),
            category: Some("production_code".to_string()),
        };

        let assessment = classifier.assess_action(&action).await.unwrap();
        assert!(!assessment.is_safe_to_execute_autonomously);
        assert_eq!(assessment.risk_level, RiskLevel::Critical);
        assert!(assessment.requires_human_approval);
    }

    #[tokio::test]
    async fn test_production_code_assessment() {
        let classifier = RuleBasedSafetyClassifier::default();
        let action = ProposedAction {
            action_type: "code_change".to_string(),
            files: vec!["src/lib.rs".to_string()],
            change: "Modify function".to_string(),
            impact: "Production code change".to_string(),
            category: Some("production_code".to_string()),
        };

        let assessment = classifier.assess_action(&action).await.unwrap();
        assert!(!assessment.is_safe_to_execute_autonomously);
        assert_eq!(assessment.risk_level, RiskLevel::High);
    }
}
