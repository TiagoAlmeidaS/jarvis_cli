//! Text chunking utilities for RAG system.

use serde::{Deserialize, Serialize};

/// Represents a chunk of text with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// Unique identifier for the chunk
    pub id: String,
    /// The text content
    pub text: String,
    /// Source document/file path
    pub source: String,
    /// Starting character position in source
    pub start_pos: usize,
    /// Ending character position in source
    pub end_pos: usize,
    /// Chunk index in the document
    pub chunk_index: usize,
    /// Metadata associated with the chunk
    pub metadata: ChunkMetadata,
}

/// Metadata for a text chunk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Title of the source document
    pub title: Option<String>,
    /// Language of the text
    pub language: Option<String>,
    /// Tags/categories
    pub tags: Vec<String>,
    /// Timestamp when chunk was created
    pub created_at: i64,
}

/// Chunking configuration.
#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    /// Maximum chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub chunk_overlap: usize,
    /// Whether to split on sentences
    pub split_on_sentences: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 500,
            chunk_overlap: 50,
            split_on_sentences: true,
        }
    }
}

/// Chunks text into smaller pieces for indexing.
pub struct TextChunker {
    config: ChunkingConfig,
}

impl TextChunker {
    /// Creates a new text chunker with configuration.
    pub fn new(config: ChunkingConfig) -> Self {
        Self { config }
    }

    /// Chunks text into smaller pieces.
    pub fn chunk_text(&self, text: &str, source: &str) -> Vec<TextChunk> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < text.len() {
            let end = (start + self.config.chunk_size).min(text.len());
            let chunk_text = if self.config.split_on_sentences && end < text.len() {
                // Try to split at sentence boundary
                self.find_sentence_boundary(&text[start..end], start)
            } else {
                end
            };

            let chunk = TextChunk {
                id: format!("{}-{}", source, chunk_index),
                text: text[start..chunk_text].to_string(),
                source: source.to_string(),
                start_pos: start,
                end_pos: chunk_text,
                chunk_index,
                metadata: ChunkMetadata {
                    created_at: Self::current_timestamp(),
                    ..Default::default()
                },
            };

            chunks.push(chunk);

            // Move start position with overlap, ensuring progress
            let overlap = self
                .config
                .chunk_overlap
                .min(chunk_text.saturating_sub(start));
            start = if chunk_text >= text.len() {
                // Last chunk, we're done
                text.len()
            } else {
                // Ensure we always advance at least 1 character to prevent infinite loops
                chunk_text.saturating_sub(overlap).max(start + 1)
            };
            chunk_index += 1;

            // Prevent infinite loop
            if start >= text.len() {
                break;
            }
        }

        chunks
    }

    /// Finds sentence boundary near the end of text.
    fn find_sentence_boundary(&self, text: &str, offset: usize) -> usize {
        let sentence_endings = ['.', '!', '?', '\n'];
        let mut best_pos = text.len();

        for &ending in &sentence_endings {
            if let Some(pos) = text.rfind(ending) {
                if pos > best_pos.min(text.len() * 3 / 4) {
                    best_pos = pos + 1;
                }
            }
        }

        offset + best_pos.min(text.len())
    }

    /// Returns current timestamp.
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

impl Default for TextChunker {
    fn default() -> Self {
        Self::new(ChunkingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let chunker = TextChunker::default();
        let text = "This is a test. ".repeat(100);
        let chunks = chunker.chunk_text(&text, "test.txt");

        assert!(!chunks.is_empty());
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_chunk_overlap() {
        let config = ChunkingConfig {
            chunk_size: 100,
            chunk_overlap: 20,
            split_on_sentences: false,
        };
        let chunker = TextChunker::new(config);
        let text = "a".repeat(200);
        let chunks = chunker.chunk_text(&text, "test.txt");

        assert!(chunks.len() >= 2);
        // Check overlap
        if chunks.len() >= 2 {
            let first_end = chunks[0].end_pos;
            let second_start = chunks[1].start_pos;
            assert!(second_start < first_end); // Overlap exists
        }
    }
}
