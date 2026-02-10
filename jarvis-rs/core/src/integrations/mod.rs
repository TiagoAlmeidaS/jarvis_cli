// Integrations module for external services
//
// This module provides integrations with external services:
// - SQL Server: Relational database for structured data persistence
// - Redis: Distributed cache for performance optimization
// - Qdrant: Vector database for RAG and semantic search

pub mod sqlserver;
pub mod redis;

#[cfg(feature = "qdrant")]
pub mod qdrant;

// Re-exports for convenience
pub use sqlserver::{Database, Repository};
pub use redis::{DistributedCache, MultiLevelCache};

#[cfg(feature = "qdrant")]
pub use qdrant::{VectorStore, QdrantVectorStore};
