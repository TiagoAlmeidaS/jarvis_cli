//! Context analysis for autonomous decision-making.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents analyzed context for decision-making.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedContext {
    /// User intent extracted from input
    pub intent: String,
    /// Key entities identified in the context
    pub entities: Vec<Entity>,
    /// Requirements extracted
    pub requirements: Vec<String>,
    /// Constraints identified
    pub constraints: Vec<String>,
    /// Goals/objectives
    pub goals: Vec<String>,
    /// Current state information
    pub current_state: HashMap<String, String>,
    /// Confidence in the analysis (0.0 to 1.0)
    pub confidence: f32,
}

/// An entity identified in the context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Entity type (file, function, service, etc.)
    pub entity_type: String,
    /// Entity name/value
    pub value: String,
    /// Confidence in entity identification
    pub confidence: f32,
}

/// Trait for context analysis.
#[async_trait::async_trait]
pub trait ContextAnalyzer: Send + Sync {
    /// Analyzes context from user input and current state.
    async fn analyze(&self, input: &str, current_state: &HashMap<String, String>) -> Result<AnalyzedContext, ContextAnalysisError>;
}

/// Error types for context analysis.
#[derive(Debug, thiserror::Error)]
pub enum ContextAnalysisError {
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Rule-based context analyzer.
///
/// This analyzer uses pattern matching and heuristics to extract context.
/// In production, this would integrate with LLM for better understanding.
pub struct RuleBasedContextAnalyzer;

impl RuleBasedContextAnalyzer {
    /// Creates a new rule-based context analyzer.
    pub fn new() -> Self {
        Self
    }

    /// Extracts entities from input.
    fn extract_entities(&self, input: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        let input_lower = input.to_lowercase();

        // Extract file references
        let file_patterns = [".rs", ".py", ".js", ".ts", ".json", ".toml"];
        for pattern in &file_patterns {
            if input_lower.contains(pattern) {
                // Simple extraction - in production, use proper parsing
                if let Some(start) = input_lower.find(pattern) {
                    let file_name = input[..start + pattern.len()].split_whitespace().last().unwrap_or("");
                    if !file_name.is_empty() {
                        entities.push(Entity {
                            entity_type: "file".to_string(),
                            value: file_name.to_string(),
                            confidence: 0.8,
                        });
                    }
                }
            }
        }

        // Extract function/method references
        if input_lower.contains("function") || input_lower.contains("fn ") || input_lower.contains("def ") {
            entities.push(Entity {
                entity_type: "function".to_string(),
                value: "function".to_string(),
                confidence: 0.6,
            });
        }

        entities
    }

    /// Extracts requirements from input.
    fn extract_requirements(&self, input: &str) -> Vec<String> {
        let mut requirements = Vec::new();
        let input_lower = input.to_lowercase();

        // Look for requirement indicators
        if input_lower.contains("need") || input_lower.contains("preciso") {
            requirements.push("User needs identified".to_string());
        }
        if input_lower.contains("must") || input_lower.contains("deve") {
            requirements.push("Must-have requirement".to_string());
        }
        if input_lower.contains("should") || input_lower.contains("deveria") {
            requirements.push("Should-have requirement".to_string());
        }

        requirements
    }

    /// Extracts constraints from input.
    fn extract_constraints(&self, input: &str) -> Vec<String> {
        let mut constraints = Vec::new();
        let input_lower = input.to_lowercase();

        // Look for constraint indicators
        if input_lower.contains("cannot") || input_lower.contains("não pode") {
            constraints.push("Cannot constraint identified".to_string());
        }
        if input_lower.contains("only") || input_lower.contains("apenas") {
            constraints.push("Only constraint identified".to_string());
        }
        if input_lower.contains("must not") || input_lower.contains("não deve") {
            constraints.push("Must-not constraint".to_string());
        }

        constraints
    }

    /// Extracts goals from input.
    fn extract_goals(&self, input: &str) -> Vec<String> {
        let mut goals = Vec::new();
        let input_lower = input.to_lowercase();

        // Look for goal indicators
        if input_lower.contains("goal") || input_lower.contains("objetivo") {
            goals.push("Goal identified".to_string());
        }
        if input_lower.contains("achieve") || input_lower.contains("alcançar") {
            goals.push("Achievement goal".to_string());
        }

        goals
    }

    /// Determines confidence in analysis.
    fn calculate_confidence(&self, entities: &[Entity], requirements: &[String]) -> f32 {
        let mut confidence: f32 = 0.5; // Base confidence

        // Increase confidence based on entities found
        if !entities.is_empty() {
            confidence += 0.2;
        }

        // Increase confidence based on requirements
        if !requirements.is_empty() {
            confidence += 0.2;
        }

        // Cap at 1.0
        confidence.min(1.0)
    }
}

impl Default for RuleBasedContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ContextAnalyzer for RuleBasedContextAnalyzer {
    async fn analyze(
        &self,
        input: &str,
        current_state: &HashMap<String, String>,
    ) -> Result<AnalyzedContext, ContextAnalysisError> {
        if input.trim().is_empty() {
            return Err(ContextAnalysisError::InvalidInput("Empty input".to_string()));
        }

        let entities = self.extract_entities(input);
        let requirements = self.extract_requirements(input);
        let constraints = self.extract_constraints(input);
        let goals = self.extract_goals(input);
        let confidence = self.calculate_confidence(&entities, &requirements);

        Ok(AnalyzedContext {
            intent: input.to_string(),
            entities,
            requirements,
            constraints,
            goals,
            current_state: current_state.clone(),
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_analysis() {
        let analyzer = RuleBasedContextAnalyzer::new();
        let state = HashMap::new();
        let context = analyzer
            .analyze("Create a REST API in Rust", &state)
            .await
            .unwrap();

        assert!(!context.intent.is_empty());
        assert!(context.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_empty_input() {
        let analyzer = RuleBasedContextAnalyzer::new();
        let state = HashMap::new();
        let result = analyzer.analyze("", &state).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_entity_extraction() {
        let analyzer = RuleBasedContextAnalyzer::new();
        let state = HashMap::new();
        let context = analyzer
            .analyze("Modify src/main.rs file", &state)
            .await
            .unwrap();

        assert!(!context.entities.is_empty());
    }
}
