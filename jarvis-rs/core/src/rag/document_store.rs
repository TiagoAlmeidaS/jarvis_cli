//! Document store for persisting indexed documents.
//!
//! This module provides functionality to persist and retrieve indexed documents
//! with their metadata and chunks.

use crate::rag::indexer::IndexedDocument;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for document storage operations.
#[async_trait]
pub trait DocumentStore: Send + Sync {
    /// Save a document to the store.
    async fn save_document(&self, doc: &IndexedDocument) -> Result<()>;

    /// Get a document by ID.
    async fn get_document(&self, id: &str) -> Result<Option<IndexedDocument>>;

    /// List all documents.
    async fn list_documents(&self) -> Result<Vec<IndexedDocument>>;

    /// Remove a document by ID.
    async fn remove_document(&self, id: &str) -> Result<()>;

    /// Check if a document exists.
    async fn exists(&self, id: &str) -> Result<bool> {
        Ok(self.get_document(id).await?.is_some())
    }

    /// Count total documents.
    async fn count(&self) -> Result<usize> {
        Ok(self.list_documents().await?.len())
    }
}

/// In-memory document store implementation.
///
/// Simple implementation for testing and fallback scenarios.
pub struct InMemoryDocumentStore {
    documents: Arc<RwLock<HashMap<String, IndexedDocument>>>,
}

impl InMemoryDocumentStore {
    /// Create a new in-memory document store.
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryDocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentStore for InMemoryDocumentStore {
    async fn save_document(&self, doc: &IndexedDocument) -> Result<()> {
        let mut docs = self.documents.write().await;
        docs.insert(doc.id.clone(), doc.clone());
        Ok(())
    }

    async fn get_document(&self, id: &str) -> Result<Option<IndexedDocument>> {
        let docs = self.documents.read().await;
        Ok(docs.get(id).cloned())
    }

    async fn list_documents(&self) -> Result<Vec<IndexedDocument>> {
        let docs = self.documents.read().await;
        Ok(docs.values().cloned().collect())
    }

    async fn remove_document(&self, id: &str) -> Result<()> {
        let mut docs = self.documents.write().await;
        docs.remove(id);
        Ok(())
    }
}

/// JSON file-based document store.
///
/// Persists documents to a JSON file on disk.
pub struct JsonFileDocumentStore {
    file_path: std::path::PathBuf,
    documents: Arc<RwLock<HashMap<String, IndexedDocument>>>,
}

impl JsonFileDocumentStore {
    /// Create a new JSON file document store.
    ///
    /// # Arguments
    /// * `file_path` - Path to the JSON file
    pub async fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Load existing documents if file exists
        let documents = if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path).await?;
            let docs_vec: Vec<IndexedDocument> = serde_json::from_str(&content)?;
            let mut map = HashMap::new();
            for doc in docs_vec {
                map.insert(doc.id.clone(), doc);
            }
            map
        } else {
            // Create parent directory if needed
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            HashMap::new()
        };

        Ok(Self {
            file_path,
            documents: Arc::new(RwLock::new(documents)),
        })
    }

    /// Save all documents to disk.
    async fn persist(&self) -> Result<()> {
        let docs = self.documents.read().await;
        let docs_vec: Vec<_> = docs.values().cloned().collect();
        let json = serde_json::to_string_pretty(&docs_vec)?;
        tokio::fs::write(&self.file_path, json).await?;
        Ok(())
    }
}

#[async_trait]
impl DocumentStore for JsonFileDocumentStore {
    async fn save_document(&self, doc: &IndexedDocument) -> Result<()> {
        {
            let mut docs = self.documents.write().await;
            docs.insert(doc.id.clone(), doc.clone());
        }
        self.persist().await
    }

    async fn get_document(&self, id: &str) -> Result<Option<IndexedDocument>> {
        let docs = self.documents.read().await;
        Ok(docs.get(id).cloned())
    }

    async fn list_documents(&self) -> Result<Vec<IndexedDocument>> {
        let docs = self.documents.read().await;
        Ok(docs.values().cloned().collect())
    }

    async fn remove_document(&self, id: &str) -> Result<()> {
        {
            let mut docs = self.documents.write().await;
            docs.remove(id);
        }
        self.persist().await
    }
}

/// PostgreSQL document store implementation.
#[cfg(feature = "postgres")]
pub mod postgres {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{PgPool, Row};

    /// PostgreSQL-based document store.
    pub struct PostgresDocumentStore {
        pool: PgPool,
    }

    impl PostgresDocumentStore {
        /// Create a new PostgreSQL document store.
        ///
        /// # Arguments
        /// * `connection_string` - PostgreSQL connection string
        pub async fn new(connection_string: &str) -> Result<Self> {
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(connection_string)
                .await?;

            // Create table if not exists
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS indexed_documents (
                    id TEXT PRIMARY KEY,
                    path TEXT NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    metadata JSONB NOT NULL,
                    indexed_at BIGINT NOT NULL,
                    chunks JSONB NOT NULL
                )
                "#,
            )
            .execute(&pool)
            .await?;

            // Create index on path for faster lookups
            sqlx::query(
                r#"
                CREATE INDEX IF NOT EXISTS idx_documents_path
                ON indexed_documents(path)
                "#,
            )
            .execute(&pool)
            .await?;

            Ok(Self { pool })
        }

        /// Create from default configuration.
        ///
        /// Uses hardcoded connection string for VPS PostgreSQL:
        /// postgresql://jarvis:jarvis_secure_password_2026@100.98.213.86:5432/jarvis
        pub async fn from_config() -> Result<Self> {
            Self::new("postgresql://jarvis:jarvis_secure_password_2026@100.98.213.86:5432/jarvis")
                .await
        }
    }

    #[async_trait]
    impl DocumentStore for PostgresDocumentStore {
        async fn save_document(&self, doc: &IndexedDocument) -> Result<()> {
            let metadata_json = serde_json::to_value(&doc.metadata)?;
            let chunks_json = serde_json::to_value(&doc.chunks)?;
            let path_str = doc.path.to_string_lossy().to_string();

            sqlx::query(
                r#"
                INSERT INTO indexed_documents (id, path, title, content, metadata, indexed_at, chunks)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (id) DO UPDATE SET
                    path = EXCLUDED.path,
                    title = EXCLUDED.title,
                    content = EXCLUDED.content,
                    metadata = EXCLUDED.metadata,
                    indexed_at = EXCLUDED.indexed_at,
                    chunks = EXCLUDED.chunks
                "#,
            )
            .bind(&doc.id)
            .bind(&path_str)
            .bind(&doc.title)
            .bind(&doc.content)
            .bind(&metadata_json)
            .bind(doc.indexed_at)
            .bind(&chunks_json)
            .execute(&self.pool)
            .await?;

            Ok(())
        }

        async fn get_document(&self, id: &str) -> Result<Option<IndexedDocument>> {
            let row = sqlx::query(
                r#"
                SELECT id, path, title, content, metadata, indexed_at, chunks
                FROM indexed_documents
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(row) = row {
                let metadata: serde_json::Value = row.get("metadata");
                let chunks: serde_json::Value = row.get("chunks");

                let doc = IndexedDocument {
                    id: row.get("id"),
                    path: std::path::PathBuf::from(row.get::<String, _>("path")),
                    title: row.get("title"),
                    content: row.get("content"),
                    metadata: serde_json::from_value(metadata)?,
                    indexed_at: row.get("indexed_at"),
                    chunks: serde_json::from_value(chunks)?,
                };

                Ok(Some(doc))
            } else {
                Ok(None)
            }
        }

        async fn list_documents(&self) -> Result<Vec<IndexedDocument>> {
            let rows = sqlx::query(
                r#"
                SELECT id, path, title, content, metadata, indexed_at, chunks
                FROM indexed_documents
                ORDER BY indexed_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await?;

            let mut documents = Vec::new();

            for row in rows {
                let metadata: serde_json::Value = row.get("metadata");
                let chunks: serde_json::Value = row.get("chunks");

                let doc = IndexedDocument {
                    id: row.get("id"),
                    path: std::path::PathBuf::from(row.get::<String, _>("path")),
                    title: row.get("title"),
                    content: row.get("content"),
                    metadata: serde_json::from_value(metadata)?,
                    indexed_at: row.get("indexed_at"),
                    chunks: serde_json::from_value(chunks)?,
                };

                documents.push(doc);
            }

            Ok(documents)
        }

        async fn remove_document(&self, id: &str) -> Result<()> {
            sqlx::query("DELETE FROM indexed_documents WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await?;

            Ok(())
        }

        async fn count(&self) -> Result<usize> {
            let row = sqlx::query("SELECT COUNT(*) as count FROM indexed_documents")
                .fetch_one(&self.pool)
                .await?;

            let count: i64 = row.get("count");
            Ok(count as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::chunk::{ChunkMetadata, TextChunk};
    use crate::rag::indexer::DocumentMetadata;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = InMemoryDocumentStore::new();

        let doc = IndexedDocument {
            id: "test_doc".to_string(),
            path: PathBuf::from("test.txt"),
            title: "Test Document".to_string(),
            content: "Test content".to_string(),
            metadata: DocumentMetadata::default(),
            indexed_at: 12345,
            chunks: vec![],
        };

        // Save
        store.save_document(&doc).await.unwrap();

        // Get
        let retrieved = store.get_document("test_doc").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Document");

        // List
        let docs = store.list_documents().await.unwrap();
        assert_eq!(docs.len(), 1);

        // Remove
        store.remove_document("test_doc").await.unwrap();
        let retrieved = store.get_document("test_doc").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_json_file_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("documents.json");

        let store = JsonFileDocumentStore::new(&file_path).await.unwrap();

        let doc = IndexedDocument {
            id: "test_doc".to_string(),
            path: PathBuf::from("test.txt"),
            title: "Test Document".to_string(),
            content: "Test content".to_string(),
            metadata: DocumentMetadata::default(),
            indexed_at: 12345,
            chunks: vec![],
        };

        // Save
        store.save_document(&doc).await.unwrap();

        // Verify file was created
        assert!(file_path.exists());

        // Create new store instance to test persistence
        let store2 = JsonFileDocumentStore::new(&file_path).await.unwrap();
        let retrieved = store2.get_document("test_doc").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Document");
    }
}
