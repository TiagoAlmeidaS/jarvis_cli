//! Learning system for extracting knowledge from interactions.

use crate::knowledge::base::{Knowledge, KnowledgeBase, KnowledgeType};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Represents a learning pattern extracted from interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPattern {
    /// Pattern identifier
    pub id: String,
    /// Pattern description
    pub description: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Conditions when pattern applies
    pub conditions: Vec<String>,
    /// Actions/behaviors associated with pattern
    pub actions: Vec<String>,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f32,
    /// Usage count
    pub usage_count: u64,
}

/// Type of learning pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Code pattern
    CodePattern,
    /// Workflow pattern
    WorkflowPattern,
    /// Problem-solving pattern
    ProblemSolvingPattern,
    /// User preference pattern
    UserPreferencePattern,
}

/// Represents an interaction to learn from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    /// Interaction ID
    pub id: String,
    /// User input
    pub user_input: String,
    /// System response
    pub system_response: String,
    /// Actions taken
    pub actions: Vec<String>,
    /// Outcome (success, failure, partial)
    pub outcome: Outcome,
    /// Timestamp
    pub timestamp: i64,
}

/// Outcome of an interaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Successful interaction
    Success,
    /// Failed interaction
    Failure,
    /// Partially successful
    Partial,
}

/// Trait for learning from interactions.
#[async_trait::async_trait]
pub trait LearningSystem: Send + Sync {
    /// Learns from an interaction.
    async fn learn_from_interaction(&self, interaction: &Interaction) -> Result<Vec<Knowledge>>;

    /// Extracts patterns from interactions.
    async fn extract_patterns(&self, interactions: &[Interaction]) -> Result<Vec<LearningPattern>>;

    /// Gets learned knowledge relevant to a query.
    async fn get_relevant_knowledge(&self, query: &str, limit: usize) -> Result<Vec<Knowledge>>;
}

/// Rule-based learning system.
pub struct RuleBasedLearningSystem {
    /// Knowledge base
    knowledge_base: Box<dyn KnowledgeBase>,
}

impl RuleBasedLearningSystem {
    /// Creates a new rule-based learning system.
    pub fn new(knowledge_base: Box<dyn KnowledgeBase>) -> Self {
        Self { knowledge_base }
    }

    /// Extracts knowledge from successful interactions.
    fn extract_knowledge_from_success(&self, interaction: &Interaction) -> Vec<Knowledge> {
        let mut knowledge = Vec::new();

        if matches!(interaction.outcome, Outcome::Success) {
            // Extract workflow pattern
            if !interaction.actions.is_empty() {
                knowledge.push(Knowledge {
                    id: format!("pattern-{}", uuid::Uuid::new_v4()),
                    content: format!(
                        "Successful workflow: {}",
                        interaction.actions.join(" -> ")
                    ),
                    knowledge_type: KnowledgeType::Pattern,
                    category: "workflow".to_string(),
                    tags: vec!["success".to_string(), "workflow".to_string()],
                    confidence: 0.8,
                    source: interaction.id.clone(),
                    created_at: interaction.timestamp,
                    last_accessed_at: interaction.timestamp,
                    access_count: 0,
                });
            }

            // Extract user preference
            if interaction.user_input.contains("prefer") || interaction.user_input.contains("gosto") {
                knowledge.push(Knowledge {
                    id: format!("preference-{}", uuid::Uuid::new_v4()),
                    content: format!("User preference: {}", interaction.user_input),
                    knowledge_type: KnowledgeType::Behavior,
                    category: "preference".to_string(),
                    tags: vec!["user_preference".to_string()],
                    confidence: 0.7,
                    source: interaction.id.clone(),
                    created_at: interaction.timestamp,
                    last_accessed_at: interaction.timestamp,
                    access_count: 0,
                });
            }
        }

        knowledge
    }

    /// Extracts knowledge from failures.
    fn extract_knowledge_from_failure(&self, interaction: &Interaction) -> Vec<Knowledge> {
        let mut knowledge = Vec::new();

        if matches!(interaction.outcome, Outcome::Failure) {
            knowledge.push(Knowledge {
                id: format!("failure-{}", uuid::Uuid::new_v4()),
                content: format!(
                    "Failed approach: {} - Avoid this pattern",
                    interaction.actions.join(" -> ")
                ),
                knowledge_type: KnowledgeType::Pattern,
                category: "failure".to_string(),
                tags: vec!["failure".to_string(), "avoid".to_string()],
                confidence: 0.9,
                source: interaction.id.clone(),
                created_at: interaction.timestamp,
                last_accessed_at: interaction.timestamp,
                access_count: 0,
            });
        }

        knowledge
    }
}

#[async_trait::async_trait]
impl LearningSystem for RuleBasedLearningSystem {
    async fn learn_from_interaction(&self, interaction: &Interaction) -> Result<Vec<Knowledge>> {
        let mut all_knowledge = Vec::new();

        // Extract knowledge from success
        all_knowledge.extend(self.extract_knowledge_from_success(interaction));

        // Extract knowledge from failure
        all_knowledge.extend(self.extract_knowledge_from_failure(interaction));

        // Store all knowledge
        for knowledge in &all_knowledge {
            self.knowledge_base.add_knowledge(knowledge.clone()).await?;
        }

        Ok(all_knowledge)
    }

    async fn extract_patterns(&self, interactions: &[Interaction]) -> Result<Vec<LearningPattern>> {
        let mut patterns = Vec::new();

        // Group interactions by action sequences
        let mut action_groups: std::collections::HashMap<String, Vec<&Interaction>> = std::collections::HashMap::new();

        for interaction in interactions {
            let action_key = interaction.actions.join("->");
            action_groups
                .entry(action_key)
                .or_insert_with(Vec::new)
                .push(interaction);
        }

        // Extract patterns from groups
        for (action_sequence, group_interactions) in action_groups {
            let success_count = group_interactions
                .iter()
                .filter(|i| matches!(i.outcome, Outcome::Success))
                .count();
            let success_rate = success_count as f32 / group_interactions.len() as f32;

            if success_rate > 0.5 {
                // Successful pattern
                patterns.push(LearningPattern {
                    id: format!("pattern-{}", uuid::Uuid::new_v4()),
                    description: format!("Successful pattern: {}", action_sequence),
                    pattern_type: PatternType::WorkflowPattern,
                    conditions: vec!["Similar user input".to_string()],
                    actions: action_sequence.split("->").map(|s| s.to_string()).collect(),
                    success_rate,
                    usage_count: group_interactions.len() as u64,
                });
            }
        }

        Ok(patterns)
    }

    async fn get_relevant_knowledge(&self, query: &str, limit: usize) -> Result<Vec<Knowledge>> {
        self.knowledge_base.search(query, limit).await.map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::base::InMemoryKnowledgeBase;

    #[tokio::test]
    async fn test_learn_from_success() {
        let kb = Box::new(InMemoryKnowledgeBase::new());
        let learning = RuleBasedLearningSystem::new(kb);

        let interaction = Interaction {
            id: "test-1".to_string(),
            user_input: "Create REST API".to_string(),
            system_response: "API created".to_string(),
            actions: vec!["generate_code".to_string(), "create_tests".to_string()],
            outcome: Outcome::Success,
            timestamp: 0,
        };

        let knowledge = learning.learn_from_interaction(&interaction).await.unwrap();
        assert!(!knowledge.is_empty());
    }

    #[tokio::test]
    async fn test_extract_patterns() {
        let kb = Box::new(InMemoryKnowledgeBase::new());
        let learning = RuleBasedLearningSystem::new(kb);

        let interactions = vec![
            Interaction {
                id: "test-1".to_string(),
                user_input: "Create API".to_string(),
                system_response: "Success".to_string(),
                actions: vec!["generate".to_string(), "test".to_string()],
                outcome: Outcome::Success,
                timestamp: 0,
            },
            Interaction {
                id: "test-2".to_string(),
                user_input: "Create API".to_string(),
                system_response: "Success".to_string(),
                actions: vec!["generate".to_string(), "test".to_string()],
                outcome: Outcome::Success,
                timestamp: 0,
            },
        ];

        let patterns = learning.extract_patterns(&interactions).await.unwrap();
        assert!(!patterns.is_empty());
    }
}
