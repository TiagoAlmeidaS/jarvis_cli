//! RAG (Retrieval Augmented Generation) system.
//!
//! This module provides functionality for document indexing, semantic search,
//! and knowledge retrieval to enhance LLM responses with contextual information.

pub mod chat_helper;
pub mod chat_integration;
pub mod chunk;
pub mod document_store;
pub mod embeddings;
pub mod indexer;
pub mod retriever;
pub mod store;

pub use chat_helper::{create_rag_injector, inject_rag_context, is_rag_ready};
pub use chat_integration::{ContextStats, RagContextConfig, RagContextInjector};
pub use chunk::{ChunkingConfig, ChunkMetadata, TextChunk, TextChunker};
pub use document_store::{DocumentStore, InMemoryDocumentStore, JsonFileDocumentStore};
pub use embeddings::{EmbeddingGenerator, OllamaEmbeddingGenerator};
pub use indexer::{DocumentIndexer, DocumentMetadata, IndexedDocument, InMemoryDocumentIndexer};
pub use retriever::{KnowledgeRetriever, RetrievedChunk, RetrievalResult, SimpleKnowledgeRetriever, SourceInfo};
pub use store::{Embedding, EmbeddingMetadata, SearchResult, VectorStore, InMemoryVectorStore};

#[cfg(feature = "qdrant")]
pub use store::qdrant::QdrantVectorStore;

#[cfg(feature = "postgres")]
pub use document_store::postgres::PostgresDocumentStore;
