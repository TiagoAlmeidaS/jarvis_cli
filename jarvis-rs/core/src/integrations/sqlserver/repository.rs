use anyhow::Result;
use async_trait::async_trait;

/// Generic repository pattern for database operations
///
/// This trait provides a standard interface for CRUD operations
/// on any entity type.
///
/// # Type Parameters
/// - `T`: The entity type (e.g., User, Conversation)
/// - `ID`: The identifier type (e.g., Uuid)
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    ID: Send + Sync + 'static,
{
    /// Find an entity by its unique identifier
    ///
    /// Returns `None` if the entity doesn't exist.
    async fn find_by_id(&self, id: ID) -> Result<Option<T>>;

    /// Find all entities
    ///
    /// Warning: This can be expensive for large tables.
    /// Consider using pagination in production.
    async fn find_all(&self) -> Result<Vec<T>>;

    /// Save a new entity
    ///
    /// The entity should have a new ID. If an entity with
    /// the same ID already exists, this may fail.
    async fn save(&self, entity: &T) -> Result<T>;

    /// Update an existing entity
    ///
    /// If the entity doesn't exist, this may fail.
    async fn update(&self, entity: &T) -> Result<T>;

    /// Delete an entity by its ID
    ///
    /// Returns an error if the entity doesn't exist or
    /// if there are foreign key constraints.
    async fn delete(&self, id: ID) -> Result<()>;

    /// Check if an entity exists
    async fn exists(&self, id: ID) -> Result<bool> {
        Ok(self.find_by_id(id).await?.is_some())
    }
}
