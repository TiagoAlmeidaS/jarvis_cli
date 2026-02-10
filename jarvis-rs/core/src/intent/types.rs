//! Types for intent detection system.

use serde::{Deserialize, Serialize};

/// Represents the type of intent detected from user input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentType {
    /// User wants to create a new skill
    CreateSkill,
    /// User wants to execute an existing skill
    ExecuteSkill,
    /// User wants to list available skills
    ListSkills,
    /// User wants to explore the codebase
    Explore,
    /// User wants to create an implementation plan
    Plan,
    /// User is asking about capabilities
    AskCapabilities,
    /// Normal chat/conversation
    NormalChat,
}

/// Represents a detected intent with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// The type of intent detected
    pub intent_type: IntentType,
    /// Confidence score between 0.0 and 1.0
    pub confidence: f32,
    /// Extracted parameters from the intent
    pub parameters: IntentParameters,
    /// Raw user input that was analyzed
    pub raw_input: String,
}

/// Parameters extracted from the intent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentParameters {
    /// Skill name (for CreateSkill or ExecuteSkill)
    pub skill_name: Option<String>,
    /// Language for skill creation (e.g., "rust", "python", "javascript")
    pub language: Option<String>,
    /// Type of skill/project (e.g., "api", "library", "component")
    pub skill_type: Option<String>,
    /// Description or requirements for the skill
    pub description: Option<String>,
    /// Query for exploration
    pub exploration_query: Option<String>,
    /// Target for planning (file, feature, etc.)
    pub planning_target: Option<String>,
}

impl Intent {
    /// Creates a new intent with the given type and confidence.
    pub fn new(intent_type: IntentType, confidence: f32, raw_input: String) -> Self {
        Self {
            intent_type,
            confidence,
            parameters: IntentParameters::default(),
            raw_input,
        }
    }

    /// Checks if the intent confidence is above the threshold.
    pub fn is_confident(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }
}
