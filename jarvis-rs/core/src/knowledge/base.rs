//! Knowledge base for accumulating and managing contextual knowledge.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a piece of knowledge in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knowledge {
    /// Unique identifier
    pub id: String,
    /// Knowledge content
    pub content: String,
    /// Knowledge type
    pub knowledge_type: KnowledgeType,
    /// Category
    pub category: String,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Confidence in the knowledge (0.0 to 1.0)
    pub confidence: f32,
    /// Source of the knowledge
    pub source: String,
    /// Timestamp when knowledge was added
    pub created_at: i64,
    /// Timestamp when knowledge was last accessed
    pub last_accessed_at: i64,
    /// Access count
    pub access_count: u64,
}

/// Type of knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeType {
    /// Factual information
    Fact,
    /// Pattern or rule
    Pattern,
    /// Best practice
    BestPractice,
    /// Learned behavior
    Behavior,
    /// Context information
    Context,
}

/// Trait for knowledge base operations.
#[async_trait::async_trait]
pub trait KnowledgeBase: Send + Sync {
    /// Adds knowledge to the base.
    async fn add_knowledge(&self, knowledge: Knowledge) -> Result<(), KnowledgeError>;

    /// Retrieves knowledge by ID.
    async fn get_knowledge(&self, id: &str) -> Result<Option<Knowledge>, KnowledgeError>;

    /// Searches knowledge by query.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Knowledge>, KnowledgeError>;

    /// Gets knowledge by category.
    async fn get_by_category(&self, category: &str) -> Result<Vec<Knowledge>, KnowledgeError>;

    /// Gets knowledge by type.
    async fn get_by_type(
        &self,
        knowledge_type: &KnowledgeType,
    ) -> Result<Vec<Knowledge>, KnowledgeError>;

    /// Updates knowledge.
    async fn update_knowledge(&self, knowledge: Knowledge) -> Result<(), KnowledgeError>;

    /// Removes knowledge.
    async fn remove_knowledge(&self, id: &str) -> Result<(), KnowledgeError>;
}

/// Error types for knowledge base operations.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    #[error("Knowledge not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Invalid knowledge data: {0}")]
    InvalidData(String),
}

/// In-memory knowledge base implementation.
pub struct InMemoryKnowledgeBase {
    /// Knowledge indexed by ID
    knowledge: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Knowledge>>>,
    /// Index by category
    by_category: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Vec<String>>>>,
    /// Index by type
    by_type: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Vec<String>>>>,
}

impl InMemoryKnowledgeBase {
    /// Creates a new in-memory knowledge base.
    pub fn new() -> Self {
        Self {
            knowledge: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            by_category: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            by_type: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Updates indexes when knowledge is added.
    async fn update_indexes(&self, knowledge: &Knowledge) {
        // Update category index
        {
            let mut by_category = self.by_category.write().await;
            by_category
                .entry(knowledge.category.clone())
                .or_insert_with(Vec::new)
                .push(knowledge.id.clone());
        }

        // Update type index
        {
            let mut by_type = self.by_type.write().await;
            let type_key = format!("{:?}", knowledge.knowledge_type);
            by_type
                .entry(type_key)
                .or_insert_with(Vec::new)
                .push(knowledge.id.clone());
        }
    }

    /// Removes from indexes when knowledge is removed.
    async fn remove_from_indexes(&self, knowledge: &Knowledge) {
        // Remove from category index
        {
            let mut by_category = self.by_category.write().await;
            if let Some(ids) = by_category.get_mut(&knowledge.category) {
                ids.retain(|id| id != &knowledge.id);
            }
        }

        // Remove from type index
        {
            let mut by_type = self.by_type.write().await;
            let type_key = format!("{:?}", knowledge.knowledge_type);
            if let Some(ids) = by_type.get_mut(&type_key) {
                ids.retain(|id| id != &knowledge.id);
            }
        }
    }

    /// Searches knowledge using simple text matching.
    fn search_knowledge(&self, knowledge: &[Knowledge], query: &str) -> Vec<Knowledge> {
        let query_lower = query.to_lowercase();
        knowledge
            .iter()
            .filter(|k| {
                k.content.to_lowercase().contains(&query_lower)
                    || k.tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
                    || k.category.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }
}

impl Default for InMemoryKnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl KnowledgeBase for InMemoryKnowledgeBase {
    async fn add_knowledge(&self, knowledge: Knowledge) -> Result<(), KnowledgeError> {
        let mut kb = self.knowledge.write().await;
        kb.insert(knowledge.id.clone(), knowledge.clone());
        drop(kb);

        self.update_indexes(&knowledge).await;
        Ok(())
    }

    async fn get_knowledge(&self, id: &str) -> Result<Option<Knowledge>, KnowledgeError> {
        let mut kb = self.knowledge.write().await;
        if let Some(knowledge) = kb.get_mut(id) {
            knowledge.last_accessed_at = Self::current_timestamp();
            knowledge.access_count += 1;
            Ok(Some(knowledge.clone()))
        } else {
            Ok(None)
        }
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Knowledge>, KnowledgeError> {
        let kb = self.knowledge.read().await;
        let all_knowledge: Vec<Knowledge> = kb.values().cloned().collect();
        drop(kb);

        let mut results = self.search_knowledge(&all_knowledge, query);

        // Sort by relevance (confidence + access count)
        results.sort_by(|a, b| {
            let score_a = a.confidence + (a.access_count as f32 / 1000.0).min(0.3);
            let score_b = b.confidence + (b.access_count as f32 / 1000.0).min(0.3);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results.into_iter().take(limit).collect())
    }

    async fn get_by_category(&self, category: &str) -> Result<Vec<Knowledge>, KnowledgeError> {
        let by_category = self.by_category.read().await;
        let ids = by_category.get(category).cloned().unwrap_or_default();
        drop(by_category);

        let kb = self.knowledge.read().await;
        let results: Vec<Knowledge> = ids.iter().filter_map(|id| kb.get(id).cloned()).collect();

        Ok(results)
    }

    async fn get_by_type(
        &self,
        knowledge_type: &KnowledgeType,
    ) -> Result<Vec<Knowledge>, KnowledgeError> {
        let by_type = self.by_type.read().await;
        let type_key = format!("{:?}", knowledge_type);
        let ids = by_type.get(&type_key).cloned().unwrap_or_default();
        drop(by_type);

        let kb = self.knowledge.read().await;
        let results: Vec<Knowledge> = ids.iter().filter_map(|id| kb.get(id).cloned()).collect();

        Ok(results)
    }

    async fn update_knowledge(&self, knowledge: Knowledge) -> Result<(), KnowledgeError> {
        let mut kb = self.knowledge.write().await;
        if kb.contains_key(&knowledge.id) {
            // Remove old indexes
            if let Some(old) = kb.get(&knowledge.id) {
                let old_clone = old.clone();
                drop(kb);
                self.remove_from_indexes(&old_clone).await;
                let mut kb = self.knowledge.write().await;
                kb.insert(knowledge.id.clone(), knowledge.clone());
                drop(kb);
                self.update_indexes(&knowledge).await;
            }
            Ok(())
        } else {
            Err(KnowledgeError::NotFound(knowledge.id))
        }
    }

    async fn remove_knowledge(&self, id: &str) -> Result<(), KnowledgeError> {
        let mut kb = self.knowledge.write().await;
        if let Some(knowledge) = kb.remove(id) {
            drop(kb);
            self.remove_from_indexes(&knowledge).await;
            Ok(())
        } else {
            Err(KnowledgeError::NotFound(id.to_string()))
        }
    }
}

impl InMemoryKnowledgeBase {
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_get_knowledge() {
        let kb = InMemoryKnowledgeBase::new();
        let knowledge = Knowledge {
            id: "test-id".to_string(),
            content: "Test knowledge".to_string(),
            knowledge_type: KnowledgeType::Fact,
            category: "test".to_string(),
            tags: vec![],
            confidence: 0.9,
            source: "test".to_string(),
            created_at: 0,
            last_accessed_at: 0,
            access_count: 0,
        };

        kb.add_knowledge(knowledge.clone()).await.unwrap();
        let retrieved = kb.get_knowledge("test-id").await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test knowledge");
    }

    #[tokio::test]
    async fn test_search_knowledge() {
        let kb = InMemoryKnowledgeBase::new();
        let knowledge = Knowledge {
            id: "test-id".to_string(),
            content: "REST API best practices".to_string(),
            knowledge_type: KnowledgeType::BestPractice,
            category: "api".to_string(),
            tags: vec!["rest".to_string(), "api".to_string()],
            confidence: 0.9,
            source: "test".to_string(),
            created_at: 0,
            last_accessed_at: 0,
            access_count: 0,
        };

        kb.add_knowledge(knowledge).await.unwrap();
        let results = kb.search("REST", 10).await.unwrap();

        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_category() {
        let kb = InMemoryKnowledgeBase::new();
        let knowledge = Knowledge {
            id: "test-id".to_string(),
            content: "Test".to_string(),
            knowledge_type: KnowledgeType::Fact,
            category: "api".to_string(),
            tags: vec![],
            confidence: 0.9,
            source: "test".to_string(),
            created_at: 0,
            last_accessed_at: 0,
            access_count: 0,
        };

        kb.add_knowledge(knowledge).await.unwrap();
        let results = kb.get_by_category("api").await.unwrap();

        assert!(!results.is_empty());
    }
}
