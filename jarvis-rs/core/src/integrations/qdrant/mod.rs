// Qdrant vector database integration module (optional feature)
//
// Provides vector storage for RAG and semantic search

#[cfg(feature = "qdrant")]
mod vector_store;

#[cfg(feature = "qdrant")]
pub use vector_store::{QdrantVectorStore, VectorStore};

// Re-export empty stubs when feature is disabled
#[cfg(not(feature = "qdrant"))]
pub struct VectorStore;

#[cfg(not(feature = "qdrant"))]
pub struct QdrantVectorStore;
