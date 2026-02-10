//! Skill evaluation service for assessing skill quality and complexity.

use crate::skills::development::{SkillDefinition, SkillDevelopmentResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Quality metrics for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillQualityMetrics {
    /// Overall quality score (0.0 to 1.0)
    pub quality_score: f32,
    /// Code complexity score (higher = more complex)
    pub complexity_score: f32,
    /// Maintainability score (0.0 to 1.0, higher = more maintainable)
    pub maintainability_score: f32,
    /// Test coverage estimate (0.0 to 1.0)
    pub test_coverage: f32,
    /// Number of lines of code
    pub lines_of_code: usize,
    /// Number of functions/methods
    pub function_count: usize,
    /// Issues found during evaluation
    pub issues: Vec<QualityIssue>,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

/// Quality issue found during evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    /// Severity of the issue (low, medium, high)
    pub severity: String,
    /// Description of the issue
    pub description: String,
    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Result of skill evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvaluationResult {
    /// Quality metrics
    pub metrics: SkillQualityMetrics,
    /// Whether the skill passed evaluation
    pub passed: bool,
    /// Minimum quality threshold
    pub threshold: f32,
}

/// Trait for skill evaluation service.
#[async_trait::async_trait]
pub trait SkillEvaluator: Send + Sync {
    /// Evaluates a skill and returns quality metrics.
    async fn evaluate_skill(&self, skill: &SkillDefinition) -> Result<SkillEvaluationResult>;
}

/// Rule-based skill evaluator.
///
/// This evaluator uses heuristics and rules to assess skill quality.
/// In production, this could be enhanced with LLM-based analysis.
pub struct RuleBasedSkillEvaluator {
    /// Minimum quality score threshold
    quality_threshold: f32,
}

impl RuleBasedSkillEvaluator {
    /// Creates a new rule-based skill evaluator.
    pub fn new(quality_threshold: f32) -> Self {
        Self {
            quality_threshold: quality_threshold.max(0.0).min(1.0),
        }
    }

    /// Calculates lines of code.
    fn count_lines(&self, code: &str) -> usize {
        code.lines().filter(|line| !line.trim().is_empty()).count()
    }

    /// Counts functions/methods in code.
    fn count_functions(&self, code: &str, language: &str) -> usize {
        match language {
            "rust" => code.matches("fn ").count() + code.matches("impl ").count(),
            "python" => code.matches("def ").count() + code.matches("class ").count(),
            "javascript" => code.matches("function ").count() + code.matches("=>").count(),
            _ => code.matches("fn ").count() + code.matches("def ").count() + code.matches("function ").count(),
        }
    }

    /// Calculates complexity score based on code structure.
    fn calculate_complexity(&self, code: &str, function_count: usize, loc: usize) -> f32 {
        // Simple complexity calculation
        // In production, use cyclomatic complexity
        let avg_functions_per_line = if loc > 0 {
            function_count as f32 / loc as f32
        } else {
            0.0
        };

        // Count control flow keywords
        let control_flow_keywords = ["if", "else", "match", "for", "while", "loop", "return"]
            .iter()
            .map(|kw| code.matches(kw).count())
            .sum::<usize>() as f32;

        // Normalize complexity (0.0 to 1.0)
        let complexity = (control_flow_keywords + avg_functions_per_line * 10.0) / 100.0;
        complexity.min(1.0)
    }

    /// Calculates maintainability score.
    fn calculate_maintainability(&self, code: &str, complexity: f32, has_tests: bool) -> f32 {
        let mut score = 1.0;

        // Reduce score based on complexity
        score -= complexity * 0.3;

        // Increase score if tests exist
        if has_tests {
            score += 0.2;
        }

        // Check for documentation/comments
        let comment_ratio = code.matches("//").count() as f32 / code.lines().count().max(1) as f32;
        if comment_ratio > 0.1 {
            score += 0.1;
        }

        score.max(0.0).min(1.0)
    }

    /// Estimates test coverage.
    fn estimate_test_coverage(&self, code: &str, test_code: Option<&str>) -> f32 {
        if test_code.is_none() {
            return 0.0;
        }

        let test_code = test_code.unwrap();
        let code_loc = self.count_lines(code);
        let test_loc = self.count_lines(test_code);

        if code_loc == 0 {
            return 0.0;
        }

        // Simple heuristic: test lines / code lines ratio
        let ratio = test_loc as f32 / code_loc as f32;
        (ratio * 0.5).min(1.0) // Cap at reasonable estimate
    }

    /// Analyzes code for issues.
    fn analyze_issues(&self, skill: &SkillDefinition) -> Vec<QualityIssue> {
        let mut issues = Vec::new();

        // Check for empty code
        if skill.code.trim().is_empty() {
            issues.push(QualityIssue {
                severity: "high".to_string(),
                description: "Skill code is empty".to_string(),
                suggestion: Some("Generate or provide skill implementation".to_string()),
            });
        }

        // Check for missing tests
        if skill.test_code.is_none() {
            issues.push(QualityIssue {
                severity: "medium".to_string(),
                description: "No test code provided".to_string(),
                suggestion: Some("Add test code to ensure skill reliability".to_string()),
            });
        }

        // Check for TODO comments (incomplete implementation)
        if skill.code.contains("TODO") || skill.code.contains("FIXME") {
            issues.push(QualityIssue {
                severity: "low".to_string(),
                description: "Code contains TODO/FIXME comments".to_string(),
                suggestion: Some("Complete implementation by addressing TODOs".to_string()),
            });
        }

        // Check code length (too short might indicate incomplete)
        let loc = self.count_lines(&skill.code);
        if loc < 10 {
            issues.push(QualityIssue {
                severity: "low".to_string(),
                description: format!("Code is very short ({} lines)", loc),
                suggestion: Some("Consider if implementation is complete".to_string()),
            });
        }

        issues
    }

    /// Generates recommendations.
    fn generate_recommendations(&self, metrics: &SkillQualityMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        if metrics.quality_score < 0.7 {
            recommendations.push("Consider improving code quality and structure".to_string());
        }

        if metrics.test_coverage < 0.5 {
            recommendations.push("Add more comprehensive tests".to_string());
        }

        if metrics.complexity_score > 0.7 {
            recommendations.push("Consider refactoring to reduce complexity".to_string());
        }

        if metrics.maintainability_score < 0.6 {
            recommendations.push("Add documentation and improve code organization".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Skill quality is good, ready for use".to_string());
        }

        recommendations
    }

    /// Calculates overall quality score.
    fn calculate_quality_score(&self, metrics: &SkillQualityMetrics) -> f32 {
        // Weighted combination of metrics
        let test_weight = 0.3;
        let maintainability_weight = 0.4;
        let complexity_weight = 0.2;
        let issues_weight = 0.1;

        let test_score = metrics.test_coverage;
        let maintainability_score = metrics.maintainability_score;
        let complexity_score = 1.0 - metrics.complexity_score.min(1.0); // Invert complexity
        let issues_score = if metrics.issues.is_empty() {
            1.0
        } else {
            let high_issues = metrics.issues.iter().filter(|i| i.severity == "high").count();
            let medium_issues = metrics.issues.iter().filter(|i| i.severity == "medium").count();
            let low_issues = metrics.issues.iter().filter(|i| i.severity == "low").count();

            // Penalize based on issue severity
            let penalty = (high_issues as f32 * 0.3) + (medium_issues as f32 * 0.15) + (low_issues as f32 * 0.05);
            (1.0 - penalty.min(0.5)).max(0.5)
        };

        (test_score * test_weight)
            + (maintainability_score * maintainability_weight)
            + (complexity_score * complexity_weight)
            + (issues_score * issues_weight)
    }
}

#[async_trait::async_trait]
impl SkillEvaluator for RuleBasedSkillEvaluator {
    async fn evaluate_skill(&self, skill: &SkillDefinition) -> Result<SkillEvaluationResult> {
        let loc = self.count_lines(&skill.code);
        let function_count = self.count_functions(&skill.code, &skill.language);
        let complexity = self.calculate_complexity(&skill.code, function_count, loc);
        let maintainability = self.calculate_maintainability(&skill.code, complexity, skill.test_code.is_some());
        let test_coverage = self.estimate_test_coverage(&skill.code, skill.test_code.as_deref());
        let issues = self.analyze_issues(skill);

        let mut metrics = SkillQualityMetrics {
            quality_score: 0.0, // Will be calculated
            complexity_score: complexity,
            maintainability_score: maintainability,
            test_coverage,
            lines_of_code: loc,
            function_count,
            issues: issues.clone(),
            recommendations: vec![],
        };

        metrics.quality_score = self.calculate_quality_score(&metrics);
        metrics.recommendations = self.generate_recommendations(&metrics);

        let passed = metrics.quality_score >= self.quality_threshold;

        Ok(SkillEvaluationResult {
            metrics,
            passed,
            threshold: self.quality_threshold,
        })
    }
}

impl Default for RuleBasedSkillEvaluator {
    fn default() -> Self {
        Self::new(0.7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evaluate_skill_with_tests() {
        let evaluator = RuleBasedSkillEvaluator::default();
        let skill = SkillDefinition {
            name: "test_skill".to_string(),
            description: "A test skill".to_string(),
            language: "rust".to_string(),
            skill_type: "api".to_string(),
            code: r#"pub fn execute() -> Result<String, String> {
    Ok("Success".to_string())
}"#
            .to_string(),
            test_code: Some(r#"#[test]
fn test_execute() {
    assert!(execute().is_ok());
}"#.to_string()),
            dependencies: vec![],
            parameters: std::collections::HashMap::new(),
            version: "1.0.0".to_string(),
        };

        let result = evaluator.evaluate_skill(&skill).await.unwrap();

        assert!(result.metrics.quality_score > 0.0);
        assert!(result.metrics.test_coverage > 0.0);
    }

    #[tokio::test]
    async fn test_evaluate_skill_without_tests() {
        let evaluator = RuleBasedSkillEvaluator::default();
        let skill = SkillDefinition {
            name: "test_skill".to_string(),
            description: "A test skill".to_string(),
            language: "rust".to_string(),
            skill_type: "api".to_string(),
            code: r#"pub fn execute() -> Result<String, String> {
    Ok("Success".to_string())
}"#
            .to_string(),
            test_code: None,
            dependencies: vec![],
            parameters: std::collections::HashMap::new(),
            version: "1.0.0".to_string(),
        };

        let result = evaluator.evaluate_skill(&skill).await.unwrap();

        assert!(result.metrics.test_coverage == 0.0);
        assert!(!result.metrics.issues.is_empty());
    }
}
