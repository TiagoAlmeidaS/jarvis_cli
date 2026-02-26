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

pub use chat_helper::create_disabled_injector_with_config;
pub use chat_helper::create_rag_injector;
pub use chat_helper::create_rag_injector_from_config;
pub use chat_helper::inject_rag_context;
pub use chat_helper::is_rag_ready;
pub use chat_integration::ContextStats;
pub use chat_integration::RagContextConfig;
pub use chat_integration::RagContextInjector;
pub use chunk::ChunkMetadata;
pub use chunk::ChunkingConfig;
pub use chunk::TextChunk;
pub use chunk::TextChunker;
pub use document_store::DocumentStore;
pub use document_store::InMemoryDocumentStore;
pub use document_store::JsonFileDocumentStore;
pub use embeddings::EmbeddingGenerator;
pub use embeddings::OllamaEmbeddingGenerator;
pub use indexer::DocumentIndexer;
pub use indexer::DocumentMetadata;
pub use indexer::InMemoryDocumentIndexer;
pub use indexer::IndexedDocument;
pub use retriever::KnowledgeRetriever;
pub use retriever::RetrievalResult;
pub use retriever::RetrievedChunk;
pub use retriever::SimpleKnowledgeRetriever;
pub use retriever::SourceInfo;
pub use store::Embedding;
pub use store::EmbeddingMetadata;
pub use store::InMemoryVectorStore;
pub use store::SearchResult;
pub use store::VectorStore;

#[cfg(feature = "qdrant")]
pub use store::qdrant::QdrantVectorStore;

#[cfg(feature = "postgres")]
pub use document_store::postgres::PostgresDocumentStore;
