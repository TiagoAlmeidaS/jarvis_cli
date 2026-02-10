//! Document indexer for RAG system.

use crate::rag::chunk::{ChunkMetadata, ChunkingConfig, TextChunk, TextChunker};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents an indexed document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    /// Document ID
    pub id: String,
    /// Document path
    pub path: PathBuf,
    /// Document title
    pub title: String,
    /// Document content
    pub content: String,
    /// Chunks from the document
    pub chunks: Vec<TextChunk>,
    /// Metadata
    pub metadata: DocumentMetadata,
    /// Timestamp when indexed
    pub indexed_at: i64,
}

/// Metadata for a document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document type (markdown, code, text, etc.)
    pub doc_type: Option<String>,
    /// Language
    pub language: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Author
    pub author: Option<String>,
}

/// Trait for document indexing.
#[async_trait::async_trait]
pub trait DocumentIndexer: Send + Sync {
    /// Indexes a document from a file path.
    async fn index_document(&self, path: &Path) -> Result<IndexedDocument>;

    /// Indexes text content directly.
    async fn index_text(&self, text: &str, source: &str, metadata: Option<DocumentMetadata>) -> Result<IndexedDocument>;

    /// Gets an indexed document by ID.
    async fn get_document(&self, id: &str) -> Result<Option<IndexedDocument>>;

    /// Lists all indexed documents.
    async fn list_documents(&self) -> Result<Vec<IndexedDocument>>;
}

/// In-memory document indexer.
pub struct InMemoryDocumentIndexer {
    /// Indexed documents
    documents: std::sync::Arc<tokio::sync::RwLock<HashMap<String, IndexedDocument>>>,
    /// Text chunker
    chunker: TextChunker,
}

impl InMemoryDocumentIndexer {
    /// Creates a new in-memory document indexer.
    pub fn new(config: ChunkingConfig) -> Self {
        Self {
            documents: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            chunker: TextChunker::new(config),
        }
    }

    /// Detects document type from file extension.
    fn detect_doc_type(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "md" | "markdown" => "markdown",
                "rs" => "rust",
                "py" => "python",
                "js" | "ts" => "javascript",
                "json" => "json",
                "toml" => "toml",
                "yaml" | "yml" => "yaml",
                _ => "text",
            })
            .map(|s| s.to_string())
    }

    /// Extracts title from content.
    fn extract_title(&self, content: &str, path: &Path) -> String {
        // Try to extract title from markdown
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with('#') {
                return first_line.trim_start_matches('#').trim().to_string();
            }
        }

        // Fallback to filename
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string()
    }
}

impl Default for InMemoryDocumentIndexer {
    fn default() -> Self {
        Self::new(ChunkingConfig::default())
    }
}

#[async_trait::async_trait]
impl DocumentIndexer for InMemoryDocumentIndexer {
    async fn index_document(&self, path: &Path) -> Result<IndexedDocument> {
        // Read file content
        let content = tokio::fs::read_to_string(path).await?;
        
        // Extract metadata
        let doc_type = self.detect_doc_type(path);
        let title = self.extract_title(&content, path);
        
        let metadata = DocumentMetadata {
            doc_type: doc_type.clone(),
            language: doc_type,
            tags: vec![],
            author: None,
        };

        // Index as text
        self.index_text(&content, &path.to_string_lossy(), Some(metadata)).await
    }

    async fn index_text(&self, text: &str, source: &str, metadata: Option<DocumentMetadata>) -> Result<IndexedDocument> {
        let doc_id = format!("doc-{}", uuid::Uuid::new_v4());
        let path = PathBuf::from(source);
        
        let title = metadata
            .as_ref()
            .and_then(|m| Some("Document".to_string()))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

        // Chunk the text
        let chunks = self.chunker.chunk_text(text, source);

        // Add metadata to chunks
        let chunks_with_metadata: Vec<TextChunk> = chunks
            .into_iter()
            .map(|mut chunk| {
                chunk.metadata.title = Some(title.clone());
                chunk.metadata.language = metadata.as_ref().and_then(|m| m.language.clone());
                chunk.metadata.tags = metadata.as_ref().map(|m| m.tags.clone()).unwrap_or_default();
                chunk
            })
            .collect();

        let document = IndexedDocument {
            id: doc_id.clone(),
            path,
            title,
            content: text.to_string(),
            chunks: chunks_with_metadata,
            metadata: metadata.unwrap_or_default(),
            indexed_at: Self::current_timestamp(),
        };

        // Store document
        let mut documents = self.documents.write().await;
        documents.insert(doc_id.clone(), document.clone());

        Ok(document)
    }

    async fn get_document(&self, id: &str) -> Result<Option<IndexedDocument>> {
        let documents = self.documents.read().await;
        Ok(documents.get(id).cloned())
    }

    async fn list_documents(&self) -> Result<Vec<IndexedDocument>> {
        let documents = self.documents.read().await;
        Ok(documents.values().cloned().collect())
    }
}

impl InMemoryDocumentIndexer {
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_index_text() {
        let indexer = InMemoryDocumentIndexer::default();
        let text = "This is a test document. ".repeat(50);
        
        let doc = indexer.index_text(&text, "test.txt", None).await.unwrap();
        
        assert!(!doc.id.is_empty());
        assert!(!doc.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_index_document() {
        let indexer = InMemoryDocumentIndexer::default();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        
        fs::write(&file_path, "# Test Document\n\nThis is a test.").unwrap();
        
        let doc = indexer.index_document(&file_path).await.unwrap();
        
        assert_eq!(doc.title, "Test Document");
        assert!(!doc.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_get_document() {
        let indexer = InMemoryDocumentIndexer::default();
        let text = "Test content";
        
        let doc = indexer.index_text(text, "test.txt", None).await.unwrap();
        let retrieved = indexer.get_document(&doc.id).await.unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, doc.id);
    }
}
