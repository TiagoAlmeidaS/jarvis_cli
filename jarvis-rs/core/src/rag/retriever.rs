//! Semantic search retriever for RAG system.

use crate::rag::chunk::TextChunk;
use crate::rag::indexer::DocumentIndexer;
use crate::rag::store::{SearchResult, VectorStore};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Result of a retrieval operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// Retrieved chunks
    pub chunks: Vec<RetrievedChunk>,
    /// Query used
    pub query: String,
    /// Number of results
    pub count: usize,
}

/// A retrieved chunk with relevance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    /// The text chunk
    pub chunk: TextChunk,
    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f32,
    /// Source document information
    pub source_info: SourceInfo,
}

/// Information about the source document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Document ID
    pub document_id: String,
    /// Document path
    pub path: String,
    /// Document title
    pub title: String,
}

/// Trait for knowledge retrieval.
#[async_trait::async_trait]
pub trait KnowledgeRetriever: Send + Sync {
    /// Retrieves relevant knowledge for a query.
    async fn retrieve(&self, query: &str, top_k: usize) -> Result<RetrievalResult>;
}

/// Simple retriever using text matching.
///
/// In production, this would use actual embeddings and vector search.
pub struct SimpleKnowledgeRetriever {
    /// Document indexer
    pub(crate) indexer: Box<dyn DocumentIndexer>,
    /// Vector store
    pub(crate) vector_store: Box<dyn VectorStore>,
    /// Minimum relevance score threshold
    min_score: f32,
}

impl SimpleKnowledgeRetriever {
    /// Creates a new simple knowledge retriever.
    pub fn new(
        indexer: Box<dyn DocumentIndexer>,
        vector_store: Box<dyn VectorStore>,
        min_score: f32,
    ) -> Self {
        Self {
            indexer,
            vector_store,
            min_score: min_score.max(0.0).min(1.0),
        }
    }

    /// Simple text-based relevance scoring.
    fn calculate_relevance(&self, chunk_text: &str, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let chunk_lower = chunk_text.to_lowercase();

        // Count keyword matches
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut matches = 0;
        let mut total_words = query_words.len().max(1);

        for word in &query_words {
            if chunk_lower.contains(word) {
                matches += 1;
            }
        }

        // Calculate score
        let base_score = matches as f32 / total_words as f32;

        // Boost score if exact phrase match
        let phrase_boost = if chunk_lower.contains(&query_lower) {
            0.3
        } else {
            0.0
        };

        (base_score + phrase_boost).min(1.0)
    }

    /// Generates a simple query vector (placeholder).
    ///
    /// In production, this would use an embedding model.
    fn generate_query_vector(&self, _query: &str) -> Vec<f32> {
        // Placeholder: return a simple vector
        // In production, use an embedding model like text-embedding-3-small
        vec![0.5; 384] // Typical embedding dimension
    }
}

#[async_trait::async_trait]
impl KnowledgeRetriever for SimpleKnowledgeRetriever {
    async fn retrieve(&self, query: &str, top_k: usize) -> Result<RetrievalResult> {
        // Get all documents
        let documents = self.indexer.list_documents().await?;

        let mut all_chunks = Vec::new();

        // Collect chunks from all documents
        for doc in documents {
            for chunk in &doc.chunks {
                // Calculate relevance score
                let relevance = self.calculate_relevance(&chunk.text, query);

                if relevance >= self.min_score {
                    all_chunks.push(RetrievedChunk {
                        chunk: chunk.clone(),
                        relevance_score: relevance,
                        source_info: SourceInfo {
                            document_id: doc.id.clone(),
                            path: doc.path.to_string_lossy().to_string(),
                            title: doc.title.clone(),
                        },
                    });
                }
            }
        }

        // Sort by relevance (descending)
        all_chunks.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top K
        let chunks = all_chunks.into_iter().take(top_k).collect();

        Ok(RetrievalResult {
            chunks,
            query: query.to_string(),
            count: top_k,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::chunk::ChunkingConfig;
    use crate::rag::indexer::{DocumentIndexer, InMemoryDocumentIndexer};
    use crate::rag::store::{InMemoryVectorStore, VectorStore};

    #[tokio::test]
    async fn test_retrieve() {
        let indexer: Box<dyn DocumentIndexer> = Box::new(InMemoryDocumentIndexer::default());
        let vector_store: Box<dyn VectorStore> = Box::new(InMemoryVectorStore::new());
        
        // Index a document first
        indexer
            .index_text("This is about REST APIs and how they work.", "test.md", None)
            .await
            .unwrap();

        // Create retriever with the same indexer
        let retriever = SimpleKnowledgeRetriever::new(indexer, vector_store, 0.1);

        // Retrieve
        let result = retriever.retrieve("REST API", 5).await.unwrap();

        assert!(!result.chunks.is_empty());
        assert!(result.chunks[0].relevance_score > 0.0);
    }

    #[tokio::test]
    async fn test_relevance_scoring() {
        let indexer = Box::new(InMemoryDocumentIndexer::default());
        let vector_store = Box::new(InMemoryVectorStore::new());
        let retriever = SimpleKnowledgeRetriever::new(indexer, vector_store, 0.0);

        let relevance = retriever.calculate_relevance(
            "This is about REST APIs",
            "REST API",
        );

        assert!(relevance > 0.0);
        assert!(relevance <= 1.0);
    }
}
