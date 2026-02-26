//! Intent detection system for analyzing user input and determining intent.

use crate::intent::types::Intent;
use crate::intent::types::IntentParameters;
use crate::intent::types::IntentType;
use anyhow::Context;
use anyhow::Result;
use std::collections::HashMap;

/// Trait for intent detection.
#[async_trait::async_trait]
pub trait IntentDetector: Send + Sync {
    /// Detects the intent from user input.
    async fn detect_intent(&self, input: &str) -> Result<Intent>;
}

/// Rule-based intent detector using pattern matching.
///
/// This is a simple implementation that uses keyword matching.
/// For production use, consider implementing an LLM-based detector.
pub struct RuleBasedIntentDetector {
    /// Minimum confidence threshold for intent detection
    confidence_threshold: f32,
}

impl RuleBasedIntentDetector {
    /// Creates a new rule-based intent detector.
    pub fn new(confidence_threshold: f32) -> Self {
        Self {
            confidence_threshold: confidence_threshold.max(0.0).min(1.0),
        }
    }

    /// Detects intent using pattern matching rules.
    fn detect_with_rules(&self, input: &str) -> Intent {
        let input_lower = input.to_lowercase();
        let mut best_intent = IntentType::NormalChat;
        let mut best_confidence = 0.0;
        let mut parameters = IntentParameters::default();

        // Pattern matching rules
        let patterns: Vec<(&str, IntentType, f32)> = vec![
            // CreateSkill patterns
            ("crie uma", IntentType::CreateSkill, 0.8),
            ("create a", IntentType::CreateSkill, 0.8),
            ("criar", IntentType::CreateSkill, 0.7),
            ("desenvolva", IntentType::CreateSkill, 0.7),
            ("build", IntentType::CreateSkill, 0.7),
            ("gerar", IntentType::CreateSkill, 0.6),
            ("generate", IntentType::CreateSkill, 0.6),
            // ExecuteSkill patterns
            ("execute", IntentType::ExecuteSkill, 0.8),
            ("executar", IntentType::ExecuteSkill, 0.8),
            ("rode", IntentType::ExecuteSkill, 0.7),
            ("run", IntentType::ExecuteSkill, 0.7),
            ("use skill", IntentType::ExecuteSkill, 0.9),
            ("usar skill", IntentType::ExecuteSkill, 0.9),
            // ListSkills patterns
            ("list skills", IntentType::ListSkills, 0.9),
            ("listar skills", IntentType::ListSkills, 0.9),
            ("quais skills", IntentType::ListSkills, 0.8),
            ("what skills", IntentType::ListSkills, 0.8),
            ("show skills", IntentType::ListSkills, 0.8),
            ("mostrar skills", IntentType::ListSkills, 0.8),
            // Explore patterns
            ("explore", IntentType::Explore, 0.8),
            ("explorar", IntentType::Explore, 0.8),
            ("analyze", IntentType::Explore, 0.7),
            ("analisar", IntentType::Explore, 0.7),
            ("understand", IntentType::Explore, 0.6),
            ("entender", IntentType::Explore, 0.6),
            // Plan patterns
            ("create plan", IntentType::Plan, 0.9),
            ("criar plano", IntentType::Plan, 0.9),
            ("plan", IntentType::Plan, 0.7),
            ("planejar", IntentType::Plan, 0.7),
            ("how to", IntentType::Plan, 0.6),
            ("como fazer", IntentType::Plan, 0.6),
            // AskCapabilities patterns
            ("what can you", IntentType::AskCapabilities, 0.9),
            ("o que você pode", IntentType::AskCapabilities, 0.9),
            ("who are you", IntentType::AskCapabilities, 0.8),
            ("quem é você", IntentType::AskCapabilities, 0.8),
            ("capabilities", IntentType::AskCapabilities, 0.7),
            ("capacidades", IntentType::AskCapabilities, 0.7),
        ];

        for (pattern, intent_type, confidence) in patterns {
            if input_lower.contains(pattern) {
                if confidence > best_confidence {
                    best_intent = intent_type.clone();
                    best_confidence = confidence;
                }
            }
        }

        // Extract parameters based on intent type
        match best_intent {
            IntentType::CreateSkill => {
                parameters = Self::extract_create_skill_params(&input_lower);
            }
            IntentType::ExecuteSkill => {
                parameters = Self::extract_execute_skill_params(&input_lower);
            }
            IntentType::Explore => {
                parameters.exploration_query = Some(input.to_string());
            }
            IntentType::Plan => {
                parameters.planning_target = Some(input.to_string());
            }
            _ => {}
        }

        Intent {
            intent_type: best_intent,
            confidence: best_confidence,
            parameters,
            raw_input: input.to_string(),
        }
    }

    /// Extracts parameters for CreateSkill intent.
    fn extract_create_skill_params(input: &str) -> IntentParameters {
        let mut params = IntentParameters::default();

        // Extract language
        let language_patterns: HashMap<&str, &str> = [
            ("rust", "rust"),
            ("python", "python"),
            ("javascript", "javascript"),
            ("typescript", "typescript"),
            ("c#", "csharp"),
            ("csharp", "csharp"),
            ("java", "java"),
            ("go", "go"),
        ]
        .iter()
        .cloned()
        .collect();

        for (pattern, lang) in &language_patterns {
            if input.contains(pattern) {
                params.language = Some(lang.to_string());
                break;
            }
        }

        // Extract skill type
        let type_patterns: HashMap<&str, &str> = [
            ("api", "api"),
            ("rest api", "api"),
            ("library", "library"),
            ("biblioteca", "library"),
            ("component", "component"),
            ("componente", "component"),
            ("script", "script"),
            ("console", "console"),
        ]
        .iter()
        .cloned()
        .collect();

        for (pattern, skill_type) in &type_patterns {
            if input.contains(pattern) {
                params.skill_type = Some(skill_type.to_string());
                break;
            }
        }

        // Extract description (everything after "crie uma" or "create a")
        if let Some(pos) = input
            .to_lowercase()
            .find("crie uma")
            .or_else(|| input.to_lowercase().find("create a"))
        {
            let desc_start = pos
                + if input[pos..].starts_with("crie uma") {
                    8
                } else {
                    8
                };
            if desc_start < input.len() {
                params.description = Some(input[desc_start..].trim().to_string());
            }
        }

        params
    }

    /// Extracts parameters for ExecuteSkill intent.
    fn extract_execute_skill_params(input: &str) -> IntentParameters {
        let mut params = IntentParameters::default();

        // Try to extract skill name after "execute", "run", "use skill"
        let patterns = [
            "execute",
            "executar",
            "run",
            "rode",
            "use skill",
            "usar skill",
        ];
        for pattern in &patterns {
            if let Some(pos) = input.to_lowercase().find(pattern) {
                let start = pos + pattern.len();
                if start < input.len() {
                    let remainder = input[start..].trim();
                    // Take first word or quoted string as skill name
                    if let Some(name) = remainder.split_whitespace().next() {
                        params.skill_name = Some(name.trim_matches('"').to_string());
                    }
                }
            }
        }

        params
    }
}

#[async_trait::async_trait]
impl IntentDetector for RuleBasedIntentDetector {
    async fn detect_intent(&self, input: &str) -> Result<Intent> {
        let intent = self.detect_with_rules(input);

        // Only return intent if confidence is above threshold
        if intent.is_confident(self.confidence_threshold) {
            Ok(intent)
        } else {
            // Return NormalChat if confidence is too low
            Ok(Intent {
                intent_type: IntentType::NormalChat,
                confidence: 1.0,
                parameters: IntentParameters::default(),
                raw_input: input.to_string(),
            })
        }
    }
}

impl Default for RuleBasedIntentDetector {
    fn default() -> Self {
        Self::new(0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_skill_intent() {
        let detector = RuleBasedIntentDetector::default();
        let intent = detector
            .detect_intent("Crie uma API REST em Rust")
            .await
            .unwrap();

        assert_eq!(intent.intent_type, IntentType::CreateSkill);
        assert!(intent.confidence > 0.5);
        assert_eq!(intent.parameters.language, Some("rust".to_string()));
        assert_eq!(intent.parameters.skill_type, Some("api".to_string()));
    }

    #[tokio::test]
    async fn test_execute_skill_intent() {
        let detector = RuleBasedIntentDetector::default();
        let intent = detector
            .detect_intent("Execute a skill MySkill")
            .await
            .unwrap();

        assert_eq!(intent.intent_type, IntentType::ExecuteSkill);
        assert!(intent.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_explore_intent() {
        let detector = RuleBasedIntentDetector::default();
        let intent = detector
            .detect_intent("Explore the codebase structure")
            .await
            .unwrap();

        assert_eq!(intent.intent_type, IntentType::Explore);
        assert!(intent.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_normal_chat() {
        let detector = RuleBasedIntentDetector::default();
        let intent = detector.detect_intent("Hello, how are you?").await.unwrap();

        assert_eq!(intent.intent_type, IntentType::NormalChat);
    }
}
