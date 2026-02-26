// Integrations module for external services
//
// This module provides integrations with external services:
// - SQL Server: Relational database for structured data persistence
// - Redis: Distributed cache for performance optimization
// - Qdrant: Vector database for RAG and semantic search

pub mod redis;
pub mod sqlserver;

#[cfg(feature = "qdrant")]
pub mod qdrant;

// Re-exports for convenience
pub use redis::DistributedCache;
pub use redis::MultiLevelCache;
pub use sqlserver::Database;
pub use sqlserver::Repository;

#[cfg(feature = "qdrant")]
pub use qdrant::QdrantVectorStore;
#[cfg(feature = "qdrant")]
pub use qdrant::VectorStore;
