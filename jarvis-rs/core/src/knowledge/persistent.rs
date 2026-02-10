//! Persistent knowledge base using file-based storage.

use crate::knowledge::base::{Knowledge, KnowledgeBase, KnowledgeError, KnowledgeType};
use std::path::PathBuf;
use tokio::fs;

/// Persistent knowledge base using JSON file storage.
pub struct PersistentKnowledgeBase {
    /// Storage directory
    storage_dir: PathBuf,
    /// In-memory fallback
    fallback: crate::knowledge::base::InMemoryKnowledgeBase,
}

impl PersistentKnowledgeBase {
    /// Creates a new persistent knowledge base.
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            fallback: crate::knowledge::base::InMemoryKnowledgeBase::new(),
        }
    }

    /// Gets the file path for knowledge.
    fn knowledge_path(&self, id: &str) -> PathBuf {
        self.storage_dir.join(format!("knowledge_{}.json", id))
    }

    /// Gets the index file path.
    fn index_path(&self) -> PathBuf {
        self.storage_dir.join("knowledge_index.json")
    }

    /// Stores knowledge to file.
    async fn store_knowledge(&self, knowledge: &Knowledge) -> Result<(), KnowledgeError> {
        // Ensure directory exists
        fs::create_dir_all(&self.storage_dir).await
            .map_err(|e| KnowledgeError::StorageError(format!("Failed to create storage dir: {}", e)))?;

        let path = self.knowledge_path(&knowledge.id);
        let data = serde_json::to_string_pretty(knowledge)
            .map_err(|e| KnowledgeError::StorageError(format!("Serialization error: {}", e)))?;
        
        fs::write(&path, data).await
            .map_err(|e| KnowledgeError::StorageError(format!("Failed to write knowledge file: {}", e)))?;
        
        // Update index
        self.update_index(knowledge).await?;
        
        Ok(())
    }

    /// Loads knowledge from file.
    async fn load_knowledge(&self, id: &str) -> Result<Option<Knowledge>, KnowledgeError> {
        let path = self.knowledge_path(id);
        
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(&path).await
            .map_err(|e| KnowledgeError::StorageError(format!("Failed to read knowledge file: {}", e)))?;
        
        let knowledge: Knowledge = serde_json::from_str(&data)
            .map_err(|e| KnowledgeError::InvalidData(format!("Deserialization error: {}", e)))?;
        
        Ok(Some(knowledge))
    }

    /// Updates the index file.
    async fn update_index(&self, knowledge: &Knowledge) -> Result<(), KnowledgeError> {
        let index_path = self.index_path();
        let mut index: Vec<String> = if index_path.exists() {
            let data = fs::read_to_string(&index_path).await
                .map_err(|e| KnowledgeError::StorageError(format!("Failed to read index: {}", e)))?;
            serde_json::from_str(&data)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if !index.contains(&knowledge.id) {
            index.push(knowledge.id.clone());
        }

        let data = serde_json::to_string_pretty(&index)
            .map_err(|e| KnowledgeError::StorageError(format!("Serialization error: {}", e)))?;
        
        fs::write(&index_path, data).await
            .map_err(|e| KnowledgeError::StorageError(format!("Failed to write index: {}", e)))?;
        
        Ok(())
    }

    /// Loads all knowledge IDs from index.
    async fn load_all_ids(&self) -> Result<Vec<String>, KnowledgeError> {
        let index_path = self.index_path();
        
        if !index_path.exists() {
            return Ok(vec![]);
        }

        let data = fs::read_to_string(&index_path).await
            .map_err(|e| KnowledgeError::StorageError(format!("Failed to read index: {}", e)))?;
        
        let ids: Vec<String> = serde_json::from_str(&data)
            .map_err(|e| KnowledgeError::InvalidData(format!("Deserialization error: {}", e)))?;
        
        Ok(ids)
    }

    /// Loads all knowledge entries.
    async fn load_all(&self) -> Result<Vec<Knowledge>, KnowledgeError> {
        let ids = self.load_all_ids().await?;
        let mut knowledge_vec = Vec::new();

        for id in ids {
            if let Some(knowledge) = self.load_knowledge(&id).await? {
                knowledge_vec.push(knowledge);
            }
        }

        Ok(knowledge_vec)
    }
}

#[async_trait::async_trait]
impl KnowledgeBase for PersistentKnowledgeBase {
    async fn add_knowledge(&self, knowledge: Knowledge) -> Result<(), KnowledgeError> {
        // Add to fallback
        self.fallback.add_knowledge(knowledge.clone()).await?;
        
        // Persist
        self.store_knowledge(&knowledge).await?;
        
        Ok(())
    }

    async fn get_knowledge(&self, id: &str) -> Result<Option<Knowledge>, KnowledgeError> {
        // Try file first
        if let Ok(Some(knowledge)) = self.load_knowledge(id).await {
            // Update fallback
            let _ = self.fallback.add_knowledge(knowledge.clone()).await;
            return Ok(Some(knowledge));
        }
        
        // Fallback to in-memory
        self.fallback.get_knowledge(id).await
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Knowledge>, KnowledgeError> {
        // Load all knowledge
        let all_knowledge = self.load_all().await?;
        
        // Search
        let query_lower = query.to_lowercase();
        let mut results: Vec<Knowledge> = all_knowledge
            .into_iter()
            .filter(|k| {
                k.content.to_lowercase().contains(&query_lower)
                    || k.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
                    || k.category.to_lowercase().contains(&query_lower)
            })
            .collect();

        // Sort by relevance
        results.sort_by(|a, b| {
            let score_a = a.confidence + (a.access_count as f32 / 1000.0).min(0.3);
            let score_b = b.confidence + (b.access_count as f32 / 1000.0).min(0.3);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results.into_iter().take(limit).collect())
    }

    async fn get_by_category(&self, category: &str) -> Result<Vec<Knowledge>, KnowledgeError> {
        let all_knowledge = self.load_all().await?;
        Ok(all_knowledge
            .into_iter()
            .filter(|k| k.category == category)
            .collect())
    }

    async fn get_by_type(&self, knowledge_type: &KnowledgeType) -> Result<Vec<Knowledge>, KnowledgeError> {
        let all_knowledge = self.load_all().await?;
        Ok(all_knowledge
            .into_iter()
            .filter(|k| &k.knowledge_type == knowledge_type)
            .collect())
    }

    async fn update_knowledge(&self, knowledge: Knowledge) -> Result<(), KnowledgeError> {
        // Update fallback
        self.fallback.update_knowledge(knowledge.clone()).await?;
        
        // Persist
        self.store_knowledge(&knowledge).await?;
        
        Ok(())
    }

    async fn remove_knowledge(&self, id: &str) -> Result<(), KnowledgeError> {
        // Remove from fallback
        self.fallback.remove_knowledge(id).await?;
        
        // Remove file
        let path = self.knowledge_path(id);
        if path.exists() {
            fs::remove_file(&path).await
                .map_err(|e| KnowledgeError::StorageError(format!("Failed to remove file: {}", e)))?;
        }
        
        // Update index
        let mut ids = self.load_all_ids().await?;
        ids.retain(|x| x != id);
        let index_path = self.index_path();
        let data = serde_json::to_string_pretty(&ids)
            .map_err(|e| KnowledgeError::StorageError(format!("Serialization error: {}", e)))?;
        fs::write(&index_path, data).await
            .map_err(|e| KnowledgeError::StorageError(format!("Failed to write index: {}", e)))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_persistent_knowledge() {
        let temp_dir = TempDir::new().unwrap();
        let kb = PersistentKnowledgeBase::new(temp_dir.path().to_path_buf());

        let knowledge = Knowledge {
            id: "test-1".to_string(),
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

        // Verify persistence
        let new_kb = PersistentKnowledgeBase::new(temp_dir.path().to_path_buf());
        let retrieved = new_kb.get_knowledge("test-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test knowledge");
    }
}
