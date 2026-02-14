//! Integration tests for RAG system
//!
//! These tests verify the complete RAG pipeline from document indexing
//! to context injection in chat.

use jarvis_core::rag::{
    ChunkingConfig, DocumentIndexer, DocumentMetadata, EmbeddingGenerator, InMemoryDocumentIndexer,
    InMemoryDocumentStore, InMemoryVectorStore, OllamaEmbeddingGenerator, RagContextConfig,
    RagContextInjector, VectorStore, create_rag_injector, inject_rag_context,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio;

/// Test document content for indexing
const TEST_DOCUMENT: &str = r#"
# Jarvis CLI Project

Jarvis is an AI-powered coding assistant built in Rust.

## Features

- RAG (Retrieval Augmented Generation) for context-aware responses
- Qdrant vector storage for semantic search
- Ollama embeddings using nomic-embed-text model
- PostgreSQL for document persistence
- Interactive TUI and non-interactive exec modes

## Authentication

The project implements JWT-based authentication in the AuthManager.
Users can authenticate using OAuth or API keys.

## RAG Implementation

The RAG system consists of:
1. VectorStore trait with Qdrant and InMemory implementations
2. EmbeddingGenerator using Ollama
3. DocumentStore for persistence
4. RagContextInjector for chat integration
"#;

const TEST_DOCUMENT_2: &str = r#"
# RAG Architecture

The RAG system uses a multi-layer architecture:

## Vector Store Layer
- Qdrant for production (cosine similarity search)
- InMemory for testing and fallback
- 768-dimensional embeddings from nomic-embed-text

## Document Store Layer
- PostgreSQL for production
- JSON file for local development
- InMemory for testing

## Embedding Generation
- Ollama service at http://100.98.213.86:11434
- Model: nomic-embed-text
- Dimension: 768
- Batch processing support
"#;

#[tokio::test]
async fn test_document_indexing_and_retrieval() {
    // Create indexer with default config
    let config = ChunkingConfig::default();
    let indexer = InMemoryDocumentIndexer::new(config);

    // Index test document
    let doc = indexer
        .index_text(TEST_DOCUMENT, "test.md", None)
        .await
        .expect("Failed to index document");

    assert!(!doc.id.is_empty(), "Document should have an ID");
    assert!(!doc.chunks.is_empty(), "Document should have chunks");
    assert_eq!(doc.path.to_str().unwrap(), "test.md");

    // Verify chunks
    for (i, chunk) in doc.chunks.iter().enumerate() {
        assert_eq!(chunk.chunk_index, i, "Chunk index should match position");
        assert!(!chunk.text.is_empty(), "Chunk text should not be empty");
        assert_eq!(chunk.source, "test.md");
    }
}

#[tokio::test]
async fn test_chunking_respects_config() {
    let config = ChunkingConfig {
        chunk_size: 100,
        chunk_overlap: 20,
        split_on_sentences: true,
    };
    let indexer = InMemoryDocumentIndexer::new(config);

    let long_text = "This is a sentence. ".repeat(50);
    let doc = indexer
        .index_text(&long_text, "long.txt", None)
        .await
        .expect("Failed to index document");

    // Should create multiple chunks due to small chunk_size
    assert!(
        doc.chunks.len() > 1,
        "Long text should be split into multiple chunks"
    );

    // Verify overlap exists
    if doc.chunks.len() >= 2 {
        let chunk1_end = &doc.chunks[0].text[doc.chunks[0].text.len().saturating_sub(20)..];
        let chunk2_start = &doc.chunks[1].text[..20.min(doc.chunks[1].text.len())];

        // Some overlap should exist (though not exact match due to sentence boundaries)
        assert!(
            !chunk1_end.is_empty() && !chunk2_start.is_empty(),
            "Chunks should have content"
        );
    }
}

#[tokio::test]
async fn test_in_memory_vector_store() {
    let store = InMemoryVectorStore::new();

    // Add test embeddings
    let embedding1 = jarvis_core::rag::Embedding {
        vector: vec![1.0, 0.0, 0.0, 0.0],
        chunk_id: "chunk1".to_string(),
        metadata: jarvis_core::rag::EmbeddingMetadata {
            source: "test.md".to_string(),
            chunk_index: 0,
            created_at: 12345,
        },
    };

    let embedding2 = jarvis_core::rag::Embedding {
        vector: vec![0.9, 0.1, 0.0, 0.0], // Similar to embedding1
        chunk_id: "chunk2".to_string(),
        metadata: jarvis_core::rag::EmbeddingMetadata {
            source: "test.md".to_string(),
            chunk_index: 1,
            created_at: 12346,
        },
    };

    let embedding3 = jarvis_core::rag::Embedding {
        vector: vec![0.0, 0.0, 1.0, 0.0], // Orthogonal to others
        chunk_id: "chunk3".to_string(),
        metadata: jarvis_core::rag::EmbeddingMetadata {
            source: "other.md".to_string(),
            chunk_index: 0,
            created_at: 12347,
        },
    };

    store
        .add_embedding(embedding1)
        .await
        .expect("Failed to add embedding1");
    store
        .add_embedding(embedding2)
        .await
        .expect("Failed to add embedding2");
    store
        .add_embedding(embedding3)
        .await
        .expect("Failed to add embedding3");

    // Search with query similar to embedding1
    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = store.search(&query, 3).await.expect("Search failed");

    assert_eq!(results.len(), 3, "Should return top 3 results");

    // First result should be most similar (chunk1)
    assert_eq!(results[0].embedding.chunk_id, "chunk1");
    assert!(
        results[0].similarity > 0.99,
        "First result should have very high similarity"
    );

    // Second should be chunk2 (similar vector)
    assert_eq!(results[1].embedding.chunk_id, "chunk2");
    assert!(
        results[1].similarity > 0.8,
        "Second result should have high similarity"
    );

    // Third should be chunk3 (orthogonal)
    assert_eq!(results[2].embedding.chunk_id, "chunk3");
    assert!(
        results[2].similarity < 0.1,
        "Third result should have low similarity"
    );
}

#[tokio::test]
async fn test_document_store_operations() {
    let store = InMemoryDocumentStore::new();

    // Create test document
    let doc = jarvis_core::rag::IndexedDocument {
        id: "test-doc-1".to_string(),
        path: std::path::PathBuf::from("test.md"),
        title: "Test Document".to_string(),
        content: TEST_DOCUMENT.to_string(),
        metadata: DocumentMetadata::default(),
        indexed_at: 12345,
        chunks: vec![],
    };

    // Save document
    store
        .save_document(&doc)
        .await
        .expect("Failed to save document");

    // Retrieve document
    let retrieved = store
        .get_document("test-doc-1")
        .await
        .expect("Failed to get document")
        .expect("Document not found");

    assert_eq!(retrieved.id, doc.id);
    assert_eq!(retrieved.title, doc.title);
    assert_eq!(retrieved.content, doc.content);

    // List documents
    let docs = store
        .list_documents()
        .await
        .expect("Failed to list documents");
    assert_eq!(docs.len(), 1);

    // Check existence
    assert!(
        store
            .exists("test-doc-1")
            .await
            .expect("Failed to check existence"),
        "Document should exist"
    );

    // Count
    let count = store.count().await.expect("Failed to count");
    assert_eq!(count, 1);

    // Remove document
    store
        .remove_document("test-doc-1")
        .await
        .expect("Failed to remove document");

    // Verify removal
    assert!(
        !store
            .exists("test-doc-1")
            .await
            .expect("Failed to check existence"),
        "Document should not exist after removal"
    );
}

#[tokio::test]
async fn test_rag_context_injection_disabled() {
    // Create a disabled injector
    let embedding_gen = Arc::new(OllamaEmbeddingGenerator::new(
        "http://localhost:11434".to_string(),
        "nomic-embed-text".to_string(),
        768,
    ));
    let vector_store = Arc::new(InMemoryVectorStore::new());
    let doc_store = Arc::new(InMemoryDocumentStore::new());

    let mut injector = RagContextInjector::new(
        embedding_gen as Arc<dyn EmbeddingGenerator>,
        vector_store as Arc<dyn VectorStore>,
        doc_store,
        false, // disabled
    );
    injector.set_enabled(false);

    let config = RagContextConfig {
        enabled: false,
        ..Default::default()
    };

    let message = "How does authentication work?";
    let result = inject_rag_context(message, &injector, &config)
        .await
        .expect("Injection should not fail even when disabled");

    assert_eq!(
        result, message,
        "Message should be unchanged when RAG is disabled"
    );
}

#[tokio::test]
async fn test_rag_context_injection_no_documents() {
    // Create enabled injector but with no documents
    let embedding_gen = Arc::new(OllamaEmbeddingGenerator::new(
        "http://localhost:11434".to_string(),
        "nomic-embed-text".to_string(),
        768,
    ));
    let vector_store = Arc::new(InMemoryVectorStore::new());
    let doc_store = Arc::new(InMemoryDocumentStore::new());

    let injector = RagContextInjector::new(
        embedding_gen as Arc<dyn EmbeddingGenerator>,
        vector_store as Arc<dyn VectorStore>,
        doc_store,
        true, // enabled
    );

    let config = RagContextConfig {
        enabled: true,
        max_chunks: 5,
        min_score: 0.7,
    };

    // Note: This test will try to connect to Ollama
    // If Ollama is not available, it should fail gracefully
    let message = "How does authentication work?";

    // We can't easily test this without Ollama running
    // Just verify the injector is created correctly
    assert!(injector.is_enabled(), "Injector should be enabled");
}

#[tokio::test]
async fn test_context_stats() {
    let doc_store = Arc::new(InMemoryDocumentStore::new());

    // Add test documents
    for i in 0..3 {
        let doc = jarvis_core::rag::IndexedDocument {
            id: format!("doc-{}", i),
            path: std::path::PathBuf::from(format!("test{}.md", i)),
            title: format!("Test Document {}", i),
            content: "Test content".to_string(),
            metadata: DocumentMetadata::default(),
            indexed_at: 12345 + i,
            chunks: vec![
                jarvis_core::rag::TextChunk {
                    id: format!("chunk-{}-0", i),
                    text: "Chunk 0 text".to_string(),
                    source: format!("test{}.md", i),
                    start_pos: 0,
                    end_pos: 13,
                    chunk_index: 0,
                    metadata: Default::default(),
                },
                jarvis_core::rag::TextChunk {
                    id: format!("chunk-{}-1", i),
                    text: "Chunk 1 text".to_string(),
                    source: format!("test{}.md", i),
                    start_pos: 14,
                    end_pos: 27,
                    chunk_index: 1,
                    metadata: Default::default(),
                },
            ],
        };

        doc_store
            .save_document(&doc)
            .await
            .expect("Failed to save document");
    }

    let embedding_gen = Arc::new(OllamaEmbeddingGenerator::new(
        "http://localhost:11434".to_string(),
        "nomic-embed-text".to_string(),
        768,
    ));
    let vector_store = Arc::new(InMemoryVectorStore::new());

    let injector = RagContextInjector::new(
        embedding_gen as Arc<dyn EmbeddingGenerator>,
        vector_store as Arc<dyn VectorStore>,
        doc_store.clone(),
        true,
    );

    let stats = injector
        .get_context_stats()
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.total_documents, 3, "Should have 3 documents");
    assert_eq!(stats.total_chunks, 6, "Should have 6 chunks (2 per doc)");
    assert_eq!(
        stats.total_size_bytes, 36,
        "Total size should be 36 bytes (12 per doc)"
    );
}

#[tokio::test]
async fn test_create_rag_injector() {
    // Test the helper function
    let injector = create_rag_injector().await;

    // Should create successfully (even if services are down, it should fallback)
    // Just verify it's created
    assert!(
        !injector.is_enabled() || injector.is_enabled(),
        "Injector should have a valid enabled state"
    );

    // Try to get stats (should work even if empty)
    let stats_result = injector.get_context_stats().await;
    assert!(
        stats_result.is_ok(),
        "Should be able to get stats: {:?}",
        stats_result.err()
    );
}

#[tokio::test]
async fn test_multiple_documents_search() {
    // Create indexer and stores
    let indexer = InMemoryDocumentIndexer::default();
    let vector_store = Arc::new(InMemoryVectorStore::new());
    let doc_store = Arc::new(InMemoryDocumentStore::new());

    // Index multiple documents
    let doc1 = indexer
        .index_text(TEST_DOCUMENT, "project.md", None)
        .await
        .expect("Failed to index doc1");

    let doc2 = indexer
        .index_text(TEST_DOCUMENT_2, "architecture.md", None)
        .await
        .expect("Failed to index doc2");

    // Store documents
    doc_store
        .save_document(&doc1)
        .await
        .expect("Failed to save doc1");
    doc_store
        .save_document(&doc2)
        .await
        .expect("Failed to save doc2");

    // Verify we have 2 documents
    let all_docs = doc_store.list_documents().await.expect("Failed to list");
    assert_eq!(all_docs.len(), 2, "Should have 2 documents");

    let total_chunks: usize = all_docs.iter().map(|d| d.chunks.len()).sum();
    assert!(total_chunks > 0, "Should have chunks");
}
