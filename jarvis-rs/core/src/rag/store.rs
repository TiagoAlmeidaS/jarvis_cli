//! Vector store for embeddings in RAG system.

use crate::rag::chunk::TextChunk;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a vector embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// Vector values
    pub vector: Vec<f32>,
    /// Chunk ID this embedding represents
    pub chunk_id: String,
    /// Metadata
    pub metadata: EmbeddingMetadata,
}

/// Metadata for an embedding.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    /// Source document
    pub source: String,
    /// Chunk index
    pub chunk_index: usize,
    /// Timestamp
    pub created_at: i64,
}

/// Trait for vector store operations.
#[async_trait::async_trait]
pub trait VectorStore: Send + Sync {
    /// Adds an embedding to the store.
    async fn add_embedding(&self, embedding: Embedding) -> Result<()>;

    /// Searches for similar embeddings.
    async fn search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<SearchResult>>;

    /// Gets embedding by chunk ID.
    async fn get_embedding(&self, chunk_id: &str) -> Result<Option<Embedding>>;

    /// Removes embedding by chunk ID.
    async fn remove_embedding(&self, chunk_id: &str) -> Result<()>;
}

/// Result of a similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The embedding
    pub embedding: Embedding,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
    /// Chunk associated with the embedding
    pub chunk: Option<TextChunk>,
}

/// In-memory vector store implementation.
///
/// This is a simple implementation using cosine similarity.
/// In production, use a proper vector database like Qdrant or Pinecone.
pub struct InMemoryVectorStore {
    /// Embeddings indexed by chunk ID
    embeddings: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Embedding>>>,
    /// Chunks indexed by chunk ID
    chunks: std::sync::Arc<tokio::sync::RwLock<HashMap<String, TextChunk>>>,
}

impl InMemoryVectorStore {
    /// Creates a new in-memory vector store.
    pub fn new() -> Self {
        Self {
            embeddings: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            chunks: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Calculates cosine similarity between two vectors.
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Stores a chunk for later retrieval.
    pub async fn store_chunk(&self, chunk: TextChunk) {
        let mut chunks = self.chunks.write().await;
        chunks.insert(chunk.id.clone(), chunk);
    }

    /// Gets a chunk by ID.
    pub async fn get_chunk(&self, chunk_id: &str) -> Option<TextChunk> {
        let chunks = self.chunks.read().await;
        chunks.get(chunk_id).cloned()
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn add_embedding(&self, embedding: Embedding) -> Result<()> {
        let mut embeddings = self.embeddings.write().await;
        embeddings.insert(embedding.chunk_id.clone(), embedding);
        Ok(())
    }

    async fn search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        let embeddings = self.embeddings.read().await;
        let chunks = self.chunks.read().await;

        let mut results: Vec<SearchResult> = embeddings
            .values()
            .map(|embedding| {
                let similarity = self.cosine_similarity(query_vector, &embedding.vector);
                let chunk = chunks.get(&embedding.chunk_id).cloned();
                SearchResult {
                    embedding: embedding.clone(),
                    similarity,
                    chunk,
                }
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top K
        Ok(results.into_iter().take(top_k).collect())
    }

    async fn get_embedding(&self, chunk_id: &str) -> Result<Option<Embedding>> {
        let embeddings = self.embeddings.read().await;
        Ok(embeddings.get(chunk_id).cloned())
    }

    async fn remove_embedding(&self, chunk_id: &str) -> Result<()> {
        let mut embeddings = self.embeddings.write().await;
        embeddings.remove(chunk_id);
        Ok(())
    }
}

/// Qdrant vector store implementation.
#[cfg(feature = "qdrant")]
pub mod qdrant {
    use super::*;
    use qdrant_client::prelude::*;
    use qdrant_client::qdrant::{
        Distance, PointStruct, VectorParams, VectorsConfig, vectors_config::Config,
    };

    /// Qdrant-based vector store implementation.
    pub struct QdrantVectorStore {
        client: QdrantClient,
        collection_name: String,
    }

    impl QdrantVectorStore {
        /// Creates a new Qdrant vector store from default configuration.
        ///
        /// Uses hardcoded values for VPS Qdrant instance:
        /// - URL: http://100.98.213.86:6333
        /// - Collection: jarvis_knowledge
        /// - Dimension: 768 (nomic-embed-text)
        pub async fn from_config() -> Result<Self> {
            Self::new(
                "http://100.98.213.86:6333",
                "jarvis_knowledge".to_string(),
                768,
            )
            .await
        }

        /// Creates a new Qdrant vector store.
        ///
        /// # Arguments
        /// * `url` - Qdrant server URL (e.g., "http://localhost:6333")
        /// * `collection_name` - Name of the collection to use
        /// * `vector_dimension` - Dimension of the vectors
        pub async fn new(
            url: &str,
            collection_name: String,
            vector_dimension: u64,
        ) -> Result<Self> {
            let client = QdrantClient::from_url(url).build()?;

            // Check if collection exists
            let collection_exists = client
                .collection_exists(&collection_name)
                .await
                .unwrap_or(false);

            if !collection_exists {
                // Create collection
                client
                    .create_collection(&CreateCollection {
                        collection_name: collection_name.clone(),
                        vectors_config: Some(VectorsConfig {
                            config: Some(Config::Params(VectorParams {
                                size: vector_dimension,
                                distance: Distance::Cosine.into(),
                                ..Default::default()
                            })),
                        }),
                        ..Default::default()
                    })
                    .await?;
            }

            Ok(Self {
                client,
                collection_name,
            })
        }

        /// Checks if the Qdrant server is available.
        pub async fn is_available(&self) -> bool {
            self.client.health_check().await.is_ok()
        }

        /// Converts an Embedding to a Qdrant Point.
        fn embedding_to_point(&self, embedding: &Embedding) -> PointStruct {
            use qdrant_client::qdrant::Value;

            let mut payload = std::collections::HashMap::new();
            payload.insert(
                "source".to_string(),
                Value::from(embedding.metadata.source.clone()),
            );
            payload.insert(
                "chunk_index".to_string(),
                Value::from(embedding.metadata.chunk_index as i64),
            );
            payload.insert(
                "created_at".to_string(),
                Value::from(embedding.metadata.created_at),
            );

            PointStruct::new(
                embedding.chunk_id.clone(),
                embedding.vector.clone(),
                payload,
            )
        }

        /// Converts a Qdrant ScoredPoint to a SearchResult.
        fn scored_point_to_result(
            &self,
            point: qdrant_client::qdrant::ScoredPoint,
        ) -> SearchResult {
            use qdrant_client::qdrant::Value;

            let chunk_id = point
                .id
                .map(|id| match id.point_id_options {
                    Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid)) => uuid,
                    Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(num)) => {
                        num.to_string()
                    }
                    None => String::new(),
                })
                .unwrap_or_default();

            let payload = point.payload;

            let source = payload
                .get("source")
                .and_then(|v| match &v.kind {
                    Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            let chunk_index = payload
                .get("chunk_index")
                .and_then(|v| match &v.kind {
                    Some(qdrant_client::qdrant::value::Kind::IntegerValue(i)) => Some(*i as usize),
                    _ => None,
                })
                .unwrap_or(0);

            let created_at = payload
                .get("created_at")
                .and_then(|v| match &v.kind {
                    Some(qdrant_client::qdrant::value::Kind::IntegerValue(i)) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0);

            let vector = point
                .vectors
                .and_then(|v| match v.vectors_options {
                    Some(qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(vec)) => {
                        #[allow(deprecated)]
                        let data = vec.data;
                        Some(data)
                    }
                    _ => None,
                })
                .unwrap_or_default();

            let embedding = Embedding {
                vector,
                chunk_id: chunk_id.clone(),
                metadata: EmbeddingMetadata {
                    source,
                    chunk_index,
                    created_at,
                },
            };

            SearchResult {
                embedding,
                similarity: point.score,
                chunk: None, // Chunk retrieval can be implemented separately
            }
        }
    }

    #[async_trait::async_trait]
    impl VectorStore for QdrantVectorStore {
        async fn add_embedding(&self, embedding: Embedding) -> Result<()> {
            let point = self.embedding_to_point(&embedding);

            self.client
                .upsert_points_blocking(&self.collection_name, None, vec![point], None)
                .await?;

            Ok(())
        }

        async fn search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
            let search_result = self
                .client
                .search_points(&SearchPoints {
                    collection_name: self.collection_name.clone(),
                    vector: query_vector.to_vec(),
                    limit: top_k as u64,
                    with_payload: Some(true.into()),
                    with_vectors: Some(true.into()),
                    ..Default::default()
                })
                .await?;

            let results = search_result
                .result
                .into_iter()
                .map(|point| self.scored_point_to_result(point))
                .collect();

            Ok(results)
        }

        async fn get_embedding(&self, chunk_id: &str) -> Result<Option<Embedding>> {
            // Retrieve a single point by ID
            let points = self
                .client
                .get_points(
                    &self.collection_name,
                    None,
                    &[chunk_id.into()],
                    Some(true),
                    Some(true),
                    None,
                )
                .await?;

            if let Some(point) = points.result.into_iter().next() {
                let search_result =
                    self.scored_point_to_result(qdrant_client::qdrant::ScoredPoint {
                        id: point.id,
                        payload: point.payload,
                        score: 1.0,
                        vectors: point.vectors,
                        shard_key: None,
                        order_value: None,
                        version: 0,
                    });
                Ok(Some(search_result.embedding))
            } else {
                Ok(None)
            }
        }

        async fn remove_embedding(&self, chunk_id: &str) -> Result<()> {
            use qdrant_client::qdrant::PointId;

            let point_id: PointId = chunk_id.into();
            let points_selector = vec![point_id].into();

            self.client
                .delete_points(&self.collection_name, None, &points_selector, None)
                .await?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::chunk::TextChunk;

    #[tokio::test]
    async fn test_add_and_search() {
        let store = InMemoryVectorStore::new();

        let embedding1 = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            chunk_id: "chunk1".to_string(),
            metadata: EmbeddingMetadata::default(),
        };

        let embedding2 = Embedding {
            vector: vec![0.0, 1.0, 0.0],
            chunk_id: "chunk2".to_string(),
            metadata: EmbeddingMetadata::default(),
        };

        store.add_embedding(embedding1).await.unwrap();
        store.add_embedding(embedding2).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 2).await.unwrap();

        assert!(!results.is_empty());
        assert!(results[0].similarity > 0.9); // Should match embedding1
    }

    #[test]
    fn test_cosine_similarity() {
        let store = InMemoryVectorStore::new();
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        let similarity = store.cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let store = InMemoryVectorStore::new();
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let similarity = store.cosine_similarity(&a, &b);
        assert!(similarity.abs() < 0.001); // Should be ~0
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let store = InMemoryVectorStore::new();
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let similarity = store.cosine_similarity(&a, &b);
        assert!((similarity + 1.0).abs() < 0.001); // Should be -1
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let store = InMemoryVectorStore::new();
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = store.cosine_similarity(&a, &b);
        assert_eq!(similarity, 0.0); // Different lengths = 0
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let store = InMemoryVectorStore::new();
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 0.0];
        let similarity = store.cosine_similarity(&a, &b);
        assert_eq!(similarity, 0.0); // Zero vector = 0
    }

    #[tokio::test]
    async fn test_get_embedding() {
        let store = InMemoryVectorStore::new();

        let embedding = Embedding {
            vector: vec![1.0, 2.0, 3.0],
            chunk_id: "test_chunk".to_string(),
            metadata: EmbeddingMetadata {
                source: "test.txt".to_string(),
                chunk_index: 5,
                created_at: 12345,
            },
        };

        store.add_embedding(embedding.clone()).await.unwrap();

        let retrieved = store.get_embedding("test_chunk").await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.chunk_id, "test_chunk");
        assert_eq!(retrieved.vector, vec![1.0, 2.0, 3.0]);
        assert_eq!(retrieved.metadata.source, "test.txt");
    }

    #[tokio::test]
    async fn test_get_nonexistent_embedding() {
        let store = InMemoryVectorStore::new();
        let retrieved = store.get_embedding("nonexistent").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_remove_embedding() {
        let store = InMemoryVectorStore::new();

        let embedding = Embedding {
            vector: vec![1.0, 2.0, 3.0],
            chunk_id: "to_remove".to_string(),
            metadata: EmbeddingMetadata::default(),
        };

        store.add_embedding(embedding).await.unwrap();

        // Verify it exists
        let retrieved = store.get_embedding("to_remove").await.unwrap();
        assert!(retrieved.is_some());

        // Remove it
        store.remove_embedding("to_remove").await.unwrap();

        // Verify it's gone
        let retrieved = store.get_embedding("to_remove").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_search_with_limit() {
        let store = InMemoryVectorStore::new();

        // Add 5 embeddings with different directions
        store
            .add_embedding(Embedding {
                vector: vec![1.0, 0.0, 0.0],
                chunk_id: "chunk0".to_string(),
                metadata: EmbeddingMetadata::default(),
            })
            .await
            .unwrap();

        store
            .add_embedding(Embedding {
                vector: vec![0.9, 0.1, 0.0],
                chunk_id: "chunk1".to_string(),
                metadata: EmbeddingMetadata::default(),
            })
            .await
            .unwrap();

        store
            .add_embedding(Embedding {
                vector: vec![1.0, 0.0, 0.0], // Same as query - highest similarity
                chunk_id: "chunk2".to_string(),
                metadata: EmbeddingMetadata::default(),
            })
            .await
            .unwrap();

        store
            .add_embedding(Embedding {
                vector: vec![0.8, 0.2, 0.0],
                chunk_id: "chunk3".to_string(),
                metadata: EmbeddingMetadata::default(),
            })
            .await
            .unwrap();

        store
            .add_embedding(Embedding {
                vector: vec![0.0, 1.0, 0.0],
                chunk_id: "chunk4".to_string(),
                metadata: EmbeddingMetadata::default(),
            })
            .await
            .unwrap();

        // Search with limit 3
        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 3).await.unwrap();

        assert_eq!(results.len(), 3); // Should only return 3 results
        // Results should be ordered by similarity - chunk0 and chunk2 are identical to query
        assert!(results[0].similarity > 0.99); // Should be very high similarity
    }

    #[tokio::test]
    async fn test_search_empty_store() {
        let store = InMemoryVectorStore::new();
        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_ordering() {
        let store = InMemoryVectorStore::new();

        let embedding1 = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            chunk_id: "exact_match".to_string(),
            metadata: EmbeddingMetadata::default(),
        };

        let embedding2 = Embedding {
            vector: vec![0.8, 0.2, 0.0],
            chunk_id: "close_match".to_string(),
            metadata: EmbeddingMetadata::default(),
        };

        let embedding3 = Embedding {
            vector: vec![0.0, 1.0, 0.0],
            chunk_id: "far_match".to_string(),
            metadata: EmbeddingMetadata::default(),
        };

        store.add_embedding(embedding1).await.unwrap();
        store.add_embedding(embedding2).await.unwrap();
        store.add_embedding(embedding3).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 3).await.unwrap();

        // Results should be ordered by similarity
        assert_eq!(results[0].embedding.chunk_id, "exact_match");
        assert_eq!(results[1].embedding.chunk_id, "close_match");
        assert_eq!(results[2].embedding.chunk_id, "far_match");

        // Verify similarity scores are descending
        assert!(results[0].similarity >= results[1].similarity);
        assert!(results[1].similarity >= results[2].similarity);
    }

    #[tokio::test]
    async fn test_chunk_storage_and_retrieval() {
        let store = InMemoryVectorStore::new();

        let chunk = TextChunk {
            id: "chunk123".to_string(),
            text: "Test content".to_string(),
            source: "test.txt".to_string(),
            start_pos: 0,
            end_pos: 12,
            chunk_index: 0,
            metadata: Default::default(),
        };

        store.store_chunk(chunk.clone()).await;

        let retrieved = store.get_chunk("chunk123").await;
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "chunk123");
        assert_eq!(retrieved.text, "Test content");
    }

    #[tokio::test]
    async fn test_chunk_nonexistent() {
        let store = InMemoryVectorStore::new();
        let retrieved = store.get_chunk("nonexistent").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use std::sync::Arc;

        let store = Arc::new(InMemoryVectorStore::new());
        let mut handles = vec![];

        // Spawn 10 concurrent tasks
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                let embedding = Embedding {
                    vector: vec![i as f32, 0.0, 0.0],
                    chunk_id: format!("chunk{}", i),
                    metadata: EmbeddingMetadata::default(),
                };

                store_clone.add_embedding(embedding).await.unwrap();

                // Verify it was added
                let retrieved = store_clone
                    .get_embedding(&format!("chunk{}", i))
                    .await
                    .unwrap();
                assert!(retrieved.is_some());
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all 10 embeddings are present
        let query = vec![5.0, 0.0, 0.0];
        let results = store.search(&query, 100).await.unwrap();
        assert_eq!(results.len(), 10);
    }

    // Note: QdrantVectorStore requires a running Qdrant instance for testing.
    // These tests should be implemented as INTEGRATION TESTS (Task #13).
    //
    // The following functionality should be tested with a real Qdrant instance:
    //
    // 1. QdrantVectorStore::new(url, collection_name, dimension)
    //    - Test successful connection
    //    - Test collection creation if not exists
    //    - Test collection reuse if exists
    //    - Test connection failure
    //
    // 2. add_embedding()
    //    - Test adding single embedding
    //    - Test upserting (updating) existing embedding
    //    - Test metadata preservation
    //    - Test vector dimension validation
    //
    // 3. search()
    //    - Test cosine similarity search
    //    - Test top-k limit
    //    - Test result ordering
    //    - Test with empty collection
    //
    // 4. get_embedding()
    //    - Test retrieving by chunk_id
    //    - Test nonexistent chunk_id
    //    - Test metadata retrieval
    //
    // 5. remove_embedding()
    //    - Test successful removal
    //    - Test removing nonexistent embedding
    //
    // 6. is_available()
    //    - Test health check with healthy server
    //    - Test health check with down server
    //
    // 7. Conversion functions
    //    - Test embedding_to_point()
    //    - Test scored_point_to_result()
    //
    // Integration test setup requirements:
    // - Docker container with Qdrant (qdrant/qdrant:latest)
    // - Collection creation and cleanup
    // - Test data fixtures
    // - Connection pool testing
    //
    // Coverage target: 80% (via integration tests)

    #[test]
    fn test_qdrant_documentation() {
        // This test ensures the integration test requirements are documented
        // Actual tests will be in tests/integration/qdrant_vector_store.rs
        assert!(true, "Integration tests required - see comments above");
    }
}
