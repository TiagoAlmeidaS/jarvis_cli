//! Chat integration for RAG system.
//!
//! This module provides functionality to inject RAG context into chat conversations.

use crate::rag::DocumentStore;
use crate::rag::EmbeddingGenerator;
use crate::rag::IndexedDocument;
use crate::rag::OllamaEmbeddingGenerator;
use crate::rag::SearchResult;
use crate::rag::VectorStore;
use anyhow::Result;
use std::sync::Arc;

#[cfg(feature = "qdrant")]
use crate::rag::QdrantVectorStore;

#[cfg(feature = "postgres")]
use crate::rag::document_store::postgres::PostgresDocumentStore;

use crate::rag::InMemoryDocumentStore;
use crate::rag::InMemoryVectorStore;
use crate::rag::JsonFileDocumentStore;

/// Context injector for RAG-enhanced chat.
pub struct RagContextInjector {
    embedding_gen: Arc<dyn EmbeddingGenerator>,
    vector_store: Arc<dyn VectorStore>,
    doc_store: Arc<dyn DocumentStore>,
    enabled: bool,
}

/// Configuration for RAG context injection.
pub struct RagContextConfig {
    /// Maximum number of chunks to retrieve
    pub max_chunks: usize,
    /// Minimum similarity score (0.0 to 1.0)
    pub min_score: f32,
    /// Whether RAG is enabled
    pub enabled: bool,
}

impl Default for RagContextConfig {
    fn default() -> Self {
        Self {
            max_chunks: 5,
            min_score: 0.7,
            enabled: true,
        }
    }
}

impl RagContextInjector {
    /// Create a new RAG context injector with custom stores.
    pub fn new(
        embedding_gen: Arc<dyn EmbeddingGenerator>,
        vector_store: Arc<dyn VectorStore>,
        doc_store: Arc<dyn DocumentStore>,
        enabled: bool,
    ) -> Self {
        Self {
            embedding_gen,
            vector_store,
            doc_store,
            enabled,
        }
    }

    /// Create from a [`RagConfig`].
    ///
    /// Attempts to use:
    /// 1. Ollama for embeddings (configured URL)
    /// 2. Qdrant for vector storage (configured URL)
    /// 3. PostgreSQL for document storage (configured URL, if set)
    ///
    /// Falls back to in-memory/JSON file storage if connections fail.
    pub async fn from_rag_config(cfg: &crate::config::types::RagConfig) -> Result<Self> {
        if !cfg.enabled {
            // Return a disabled injector using in-memory stores
            let embedding_gen = Arc::new(OllamaEmbeddingGenerator::from_rag_config(cfg));
            let vector_store: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new());
            let doc_store: Arc<dyn DocumentStore> = Arc::new(InMemoryDocumentStore::new());
            return Ok(Self {
                embedding_gen,
                vector_store,
                doc_store,
                enabled: false,
            });
        }

        let embedding_gen = Arc::new(OllamaEmbeddingGenerator::from_rag_config(cfg));

        // Try Qdrant, fallback to in-memory
        #[cfg(feature = "qdrant")]
        let vector_store: Arc<dyn VectorStore> = QdrantVectorStore::from_rag_config(cfg)
            .await
            .map(|s| Arc::new(s) as Arc<dyn VectorStore>)
            .unwrap_or_else(|_| Arc::new(InMemoryVectorStore::new()));

        #[cfg(not(feature = "qdrant"))]
        let vector_store: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new());

        // Try PostgreSQL (if configured), fallback to JSON file
        #[cfg(feature = "postgres")]
        let doc_store: Arc<dyn DocumentStore> = if cfg.postgres_url.is_some() {
            match PostgresDocumentStore::from_rag_config(cfg).await {
                Ok(s) => Arc::new(s),
                Err(_) => Self::fallback_doc_store().await,
            }
        } else {
            Self::fallback_doc_store().await
        };

        #[cfg(not(feature = "postgres"))]
        let doc_store: Arc<dyn DocumentStore> = Self::fallback_doc_store().await;

        Ok(Self {
            embedding_gen,
            vector_store,
            doc_store,
            enabled: true,
        })
    }

    /// Create from default configuration (legacy).
    ///
    /// Attempts to use:
    /// 1. Ollama for embeddings (localhost:11434)
    /// 2. Qdrant for vector storage (localhost:6333)
    /// 3. PostgreSQL for document storage (localhost:5432)
    ///
    /// Falls back to in-memory/JSON file storage if connections fail.
    pub async fn from_config() -> Result<Self> {
        let embedding_gen = Arc::new(OllamaEmbeddingGenerator::from_config()?);

        // Try Qdrant, fallback to in-memory
        #[cfg(feature = "qdrant")]
        let vector_store: Arc<dyn VectorStore> = QdrantVectorStore::from_config()
            .await
            .map(|s| Arc::new(s) as Arc<dyn VectorStore>)
            .unwrap_or_else(|_| Arc::new(InMemoryVectorStore::new()));

        #[cfg(not(feature = "qdrant"))]
        let vector_store: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new());

        // Try PostgreSQL, fallback to JSON file / in-memory
        #[cfg(feature = "postgres")]
        let doc_store: Arc<dyn DocumentStore> = match PostgresDocumentStore::from_config().await {
            Ok(s) => Arc::new(s),
            Err(_) => Self::fallback_doc_store().await,
        };

        #[cfg(not(feature = "postgres"))]
        let doc_store: Arc<dyn DocumentStore> = Self::fallback_doc_store().await;

        Ok(Self {
            embedding_gen,
            vector_store,
            doc_store,
            enabled: true,
        })
    }

    /// Fallback document store: JSON file in jarvis_home, or in-memory.
    async fn fallback_doc_store() -> Arc<dyn DocumentStore> {
        let json_path = jarvis_utils_home_dir::find_jarvis_home()
            .ok()
            .map(|h| h.join("documents.json"))
            .unwrap_or_else(|| std::path::PathBuf::from(".jarvis/documents.json"));

        match JsonFileDocumentStore::new(&json_path).await {
            Ok(s) => Arc::new(s),
            Err(_) => Arc::new(InMemoryDocumentStore::new()),
        }
    }

    /// Check if RAG is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable RAG context injection.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get relevant context for a user query.
    ///
    /// Returns a formatted string containing relevant document chunks,
    /// ready to be prepended to the chat prompt.
    pub async fn get_relevant_context(
        &self,
        query: &str,
        config: &RagContextConfig,
    ) -> Result<String> {
        if !self.enabled || !config.enabled {
            return Ok(String::new());
        }

        // Generate embedding for the query
        let query_embedding = self.embedding_gen.generate_embedding(query).await?;

        // Search for relevant chunks
        let search_results = self
            .vector_store
            .search(&query_embedding, config.max_chunks)
            .await?;

        // Filter by minimum score
        let relevant_results: Vec<_> = search_results
            .into_iter()
            .filter(|r| r.similarity >= config.min_score)
            .collect();

        if relevant_results.is_empty() {
            return Ok(String::new());
        }

        // Build context string
        let mut context = String::from("\n\n## Relevant Context from Project Files\n\n");
        context.push_str("The following information was retrieved from your local project files and may be relevant to your question:\n\n");

        for (i, result) in relevant_results.iter().enumerate() {
            // Try to get the full document to retrieve chunk text
            if let Ok(Some(doc)) = self
                .doc_store
                .get_document(&result.embedding.metadata.source)
                .await
            {
                // Find the chunk in the document
                if let Some(chunk) = doc
                    .chunks
                    .iter()
                    .find(|c| c.id == result.embedding.chunk_id)
                {
                    context.push_str(&format!(
                        "### Snippet {} (from `{}`, relevance: {:.0}%)\n\n",
                        i + 1,
                        result.embedding.metadata.source,
                        result.similarity * 100.0
                    ));

                    context.push_str("```\n");
                    context.push_str(&chunk.text);
                    context.push_str("\n```\n\n");
                }
            }
        }

        context.push_str("---\n\n");
        context.push_str("Please use the above context when answering the user's question, but only if it's relevant. Don't mention the context explicitly unless asked.\n\n");

        Ok(context)
    }

    /// Get a summary of available context (for debugging/stats).
    pub async fn get_context_stats(&self) -> Result<ContextStats> {
        let docs = self.doc_store.list_documents().await?;

        let total_chunks: usize = docs.iter().map(|d| d.chunks.len()).sum();
        let total_size: usize = docs.iter().map(|d| d.content.len()).sum();

        Ok(ContextStats {
            total_documents: docs.len(),
            total_chunks,
            total_size_bytes: total_size,
        })
    }
}

/// Statistics about available RAG context.
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub total_documents: usize,
    pub total_chunks: usize,
    pub total_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::ChunkMetadata;
    use crate::rag::ChunkingConfig;
    use crate::rag::DocumentMetadata;
    use crate::rag::InMemoryDocumentIndexer;
    use crate::rag::TextChunk;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_context_injector_disabled() {
        let embedding_gen = Arc::new(OllamaEmbeddingGenerator::from_config().unwrap());
        let vector_store = Arc::new(InMemoryVectorStore::new());
        let doc_store = Arc::new(InMemoryDocumentStore::new());

        let injector = RagContextInjector::new(embedding_gen, vector_store, doc_store, false);

        let config = RagContextConfig::default();
        let context = injector
            .get_relevant_context("test query", &config)
            .await
            .unwrap();

        assert!(context.is_empty(), "Context should be empty when disabled");
    }

    #[tokio::test]
    async fn test_context_stats() {
        let doc_store = Arc::new(InMemoryDocumentStore::new());

        // Add a test document
        let test_doc = crate::rag::IndexedDocument {
            id: "test_doc".to_string(),
            path: PathBuf::from("test.txt"),
            title: "Test Document".to_string(),
            content: "Test content".to_string(),
            metadata: DocumentMetadata::default(),
            indexed_at: 12345,
            chunks: vec![TextChunk {
                id: "chunk1".to_string(),
                text: "Test chunk".to_string(),
                source: "test.txt".to_string(),
                start_pos: 0,
                end_pos: 10,
                chunk_index: 0,
                metadata: ChunkMetadata::default(),
            }],
        };

        doc_store.save_document(&test_doc).await.unwrap();

        let embedding_gen = Arc::new(OllamaEmbeddingGenerator::from_config().unwrap());
        let vector_store = Arc::new(InMemoryVectorStore::new());

        let injector =
            RagContextInjector::new(embedding_gen, vector_store, doc_store.clone(), true);

        let stats = injector.get_context_stats().await.unwrap();

        assert_eq!(stats.total_documents, 1);
        assert_eq!(stats.total_chunks, 1);
    }
}
