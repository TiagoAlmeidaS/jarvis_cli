//! RAG (Retrieval Augmented Generation) system.
//!
//! This module provides functionality for document indexing, semantic search,
//! and knowledge retrieval to enhance LLM responses with contextual information.

pub mod chunk;
pub mod indexer;
pub mod retriever;
pub mod store;

pub use chunk::{ChunkingConfig, ChunkMetadata, TextChunk, TextChunker};
pub use indexer::{DocumentIndexer, DocumentMetadata, IndexedDocument, InMemoryDocumentIndexer};
pub use retriever::{KnowledgeRetriever, RetrievedChunk, RetrievalResult, SimpleKnowledgeRetriever, SourceInfo};
pub use store::{Embedding, EmbeddingMetadata, SearchResult, VectorStore, InMemoryVectorStore};

#[cfg(feature = "qdrant")]
pub use store::qdrant::QdrantVectorStore;
