//! Risk assessment for autonomous actions.

use serde::{Deserialize, Serialize};

/// Risk level for an action.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Low risk - safe for autonomous execution
    Low,
    /// Medium risk - requires consideration
    Medium,
    /// High risk - should require human approval
    High,
    /// Critical risk - must not execute autonomously
    Critical,
}

/// Safety assessment for a proposed action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyAssessment {
    /// Whether the action is safe to execute autonomously
    pub is_safe_to_execute_autonomously: bool,
    /// Risk level of the action
    pub risk_level: RiskLevel,
    /// Reasoning for the assessment
    pub reasoning: String,
    /// Whether human approval is required
    pub requires_human_approval: bool,
    /// Safety checks performed
    pub safety_checks: Vec<String>,
    /// Confidence in the assessment (0.0 to 1.0)
    pub confidence: f32,
}

impl SafetyAssessment {
    /// Creates a safe assessment.
    pub fn safe(reasoning: String) -> Self {
        Self {
            is_safe_to_execute_autonomously: true,
            risk_level: RiskLevel::Low,
            reasoning,
            requires_human_approval: false,
            safety_checks: vec!["Basic safety checks passed".to_string()],
            confidence: 0.9,
        }
    }

    /// Creates an unsafe assessment.
    pub fn r#unsafe(risk_level: RiskLevel, reasoning: String) -> Self {
        Self {
            is_safe_to_execute_autonomously: false,
            risk_level,
            reasoning,
            requires_human_approval: true,
            safety_checks: vec![],
            confidence: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_assessment() {
        let assessment = SafetyAssessment::safe("Test file modification".to_string());
        assert!(assessment.is_safe_to_execute_autonomously);
        assert_eq!(assessment.risk_level, RiskLevel::Low);
        assert!(!assessment.requires_human_approval);
    }

    #[test]
    fn test_unsafe_assessment() {
        let assessment =
            SafetyAssessment::r#unsafe(RiskLevel::High, "Production code modification".to_string());
        assert!(!assessment.is_safe_to_execute_autonomously);
        assert_eq!(assessment.risk_level, RiskLevel::High);
        assert!(assessment.requires_human_approval);
    }
}
