//! Capability registry for managing and discovering system capabilities.

use crate::capability::metadata::CapabilityMetadata;
use crate::capability::metadata::CapabilityType;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for capability registry operations.
#[async_trait::async_trait]
pub trait CapabilityRegistry: Send + Sync {
    /// Registers a capability in the registry.
    async fn register(&self, capability: CapabilityMetadata) -> Result<()>;

    /// Retrieves a capability by ID.
    async fn get(&self, id: &str) -> Result<Option<CapabilityMetadata>>;

    /// Retrieves a capability by name.
    async fn get_by_name(&self, name: &str) -> Result<Option<CapabilityMetadata>>;

    /// Lists all capabilities.
    async fn list(&self) -> Result<Vec<CapabilityMetadata>>;

    /// Lists capabilities by type.
    async fn list_by_type(
        &self,
        capability_type: &CapabilityType,
    ) -> Result<Vec<CapabilityMetadata>>;

    /// Searches capabilities by query.
    async fn search(&self, query: &str) -> Result<Vec<CapabilityMetadata>>;

    /// Gets dependencies for a capability.
    async fn get_dependencies(&self, id: &str) -> Result<Vec<CapabilityMetadata>>;

    /// Checks if a capability exists.
    async fn exists(&self, id: &str) -> bool;

    /// Updates a capability.
    async fn update(&self, capability: CapabilityMetadata) -> Result<()>;

    /// Removes a capability.
    async fn remove(&self, id: &str) -> Result<()>;
}

/// In-memory implementation of capability registry.
pub struct InMemoryCapabilityRegistry {
    /// Capabilities indexed by ID
    by_id: Arc<RwLock<HashMap<String, CapabilityMetadata>>>,
    /// Capabilities indexed by name
    by_name: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryCapabilityRegistry {
    /// Creates a new in-memory capability registry.
    pub fn new() -> Self {
        Self {
            by_id: Arc::new(RwLock::new(HashMap::new())),
            by_name: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Searches capabilities by text matching.
    fn search_capabilities(
        &self,
        capabilities: &[CapabilityMetadata],
        query: &str,
    ) -> Vec<CapabilityMetadata> {
        let query_lower = query.to_lowercase();
        capabilities
            .iter()
            .filter(|cap| {
                cap.name.to_lowercase().contains(&query_lower)
                    || cap.description.to_lowercase().contains(&query_lower)
                    || cap
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }
}

impl Default for InMemoryCapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CapabilityRegistry for InMemoryCapabilityRegistry {
    async fn register(&self, capability: CapabilityMetadata) -> Result<()> {
        let id = capability.id.clone();
        let name = capability.name.clone();

        let mut by_id = self.by_id.write().await;
        let mut by_name = self.by_name.write().await;

        by_id.insert(id.clone(), capability.clone());
        by_name.insert(name, id);

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<CapabilityMetadata>> {
        let by_id = self.by_id.read().await;
        Ok(by_id.get(id).cloned())
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<CapabilityMetadata>> {
        let by_name = self.by_name.read().await;
        if let Some(id) = by_name.get(name) {
            self.get(id).await
        } else {
            Ok(None)
        }
    }

    async fn list(&self) -> Result<Vec<CapabilityMetadata>> {
        let by_id = self.by_id.read().await;
        Ok(by_id.values().cloned().collect())
    }

    async fn list_by_type(
        &self,
        capability_type: &CapabilityType,
    ) -> Result<Vec<CapabilityMetadata>> {
        let by_id = self.by_id.read().await;
        Ok(by_id
            .values()
            .filter(|cap| &cap.capability_type == capability_type)
            .cloned()
            .collect())
    }

    async fn search(&self, query: &str) -> Result<Vec<CapabilityMetadata>> {
        let capabilities = self.list().await?;
        Ok(self.search_capabilities(&capabilities, query))
    }

    async fn get_dependencies(&self, id: &str) -> Result<Vec<CapabilityMetadata>> {
        let by_id = self.by_id.read().await;
        if let Some(capability) = by_id.get(id) {
            let mut dependencies = Vec::new();
            for dep_id in &capability.dependencies {
                if let Some(dep) = by_id.get(dep_id) {
                    dependencies.push(dep.clone());
                }
            }
            Ok(dependencies)
        } else {
            Ok(vec![])
        }
    }

    async fn exists(&self, id: &str) -> bool {
        let by_id = self.by_id.read().await;
        by_id.contains_key(id)
    }

    async fn update(&self, capability: CapabilityMetadata) -> Result<()> {
        let id = capability.id.clone();
        let name = capability.name.clone();

        let mut by_id = self.by_id.write().await;
        let mut by_name = self.by_name.write().await;

        if by_id.contains_key(&id) {
            // Update name mapping if name changed
            if let Some(old_cap) = by_id.get(&id) {
                if old_cap.name != name {
                    by_name.remove(&old_cap.name);
                    by_name.insert(name.clone(), id.clone());
                }
            }

            by_id.insert(id, capability);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Capability not found"))
        }
    }

    async fn remove(&self, id: &str) -> Result<()> {
        let mut by_id = self.by_id.write().await;
        let mut by_name = self.by_name.write().await;

        if let Some(capability) = by_id.remove(id) {
            by_name.remove(&capability.name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Capability not found"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::metadata::CapabilityMetadata;

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = InMemoryCapabilityRegistry::new();
        let capability = CapabilityMetadata::new(
            "test-id".to_string(),
            "test-capability".to_string(),
            CapabilityType::Tool,
            "A test capability".to_string(),
        );

        registry.register(capability.clone()).await.unwrap();
        let retrieved = registry.get("test-id").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-capability");
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let registry = InMemoryCapabilityRegistry::new();
        let capability = CapabilityMetadata::new(
            "test-id".to_string(),
            "test-capability".to_string(),
            CapabilityType::Tool,
            "A test capability".to_string(),
        );

        registry.register(capability).await.unwrap();
        let retrieved = registry.get_by_name("test-capability").await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_search() {
        let registry = InMemoryCapabilityRegistry::new();
        let capability = CapabilityMetadata::new(
            "test-id".to_string(),
            "test-capability".to_string(),
            CapabilityType::Tool,
            "A test capability for searching".to_string(),
        );

        registry.register(capability).await.unwrap();
        let results = registry.search("test").await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_list_by_type() {
        let registry = InMemoryCapabilityRegistry::new();
        let tool = CapabilityMetadata::new(
            "tool-id".to_string(),
            "tool".to_string(),
            CapabilityType::Tool,
            "A tool".to_string(),
        );
        let service = CapabilityMetadata::new(
            "service-id".to_string(),
            "service".to_string(),
            CapabilityType::Service,
            "A service".to_string(),
        );

        registry.register(tool).await.unwrap();
        registry.register(service).await.unwrap();

        let tools = registry.list_by_type(&CapabilityType::Tool).await.unwrap();
        assert_eq!(tools.len(), 1);
    }
}
