//! Embedding generation for RAG system.
//!
//! This module provides functionality to generate vector embeddings from text
//! using various embedding models (primarily Ollama).

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;

/// Trait for generating text embeddings.
#[async_trait]
pub trait EmbeddingGenerator: Send + Sync {
    /// Generate embedding for a single text.
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts in batch.
    async fn generate_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    /// Get the dimension of embeddings produced by this generator.
    fn embedding_dimension(&self) -> usize;
}

/// Ollama embedding generator using the Ollama API.
pub struct OllamaEmbeddingGenerator {
    client: Client,
    base_url: String,
    model: String,
    dimension: usize,
}

#[derive(Serialize)]
struct OllamaEmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

impl OllamaEmbeddingGenerator {
    /// Create a new Ollama embedding generator.
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the Ollama server (e.g., "http://100.98.213.86:11434")
    /// * `model` - Model name to use (e.g., "nomic-embed-text")
    /// * `dimension` - Expected embedding dimension (768 for nomic-embed-text)
    pub fn new(base_url: String, model: String, dimension: usize) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url,
            model,
            dimension,
        }
    }

    /// Create from a [`RagConfig`].
    ///
    /// Reads the Ollama URL, model name, and embedding dimension from the
    /// supplied configuration instead of using hardcoded values.
    pub fn from_rag_config(cfg: &crate::config::types::RagConfig) -> Self {
        Self::new(
            cfg.ollama_url.clone(),
            cfg.ollama_model.clone(),
            cfg.embedding_dimension,
        )
    }

    /// Create from default configuration (legacy).
    ///
    /// Uses localhost defaults:
    /// - URL: http://localhost:11434
    /// - Model: nomic-embed-text
    /// - Dimension: 768
    pub fn from_config() -> Result<Self> {
        Ok(Self::new(
            "http://localhost:11434".to_string(),
            "nomic-embed-text".to_string(),
            768, // nomic-embed-text dimension
        ))
    }

    /// Create for testing with custom settings.
    #[cfg(test)]
    pub fn for_testing(base_url: String) -> Self {
        Self::new(base_url, "nomic-embed-text".to_string(), 768)
    }
}

#[async_trait]
impl EmbeddingGenerator for OllamaEmbeddingGenerator {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);
        let request = OllamaEmbedRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request to Ollama: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!(
                "Ollama embedding request failed with status {}: {}",
                status,
                error_text
            );
        }

        let embed_response: OllamaEmbedResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Ollama response: {}", e))?;

        // Validate embedding dimension
        if embed_response.embedding.len() != self.dimension {
            tracing::warn!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.dimension,
                embed_response.embedding.len()
            );
        }

        Ok(embed_response.embedding)
    }

    async fn generate_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let embedding = self.generate_embedding(&text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    fn embedding_dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_generator_creation() {
        let generator = OllamaEmbeddingGenerator::new(
            "http://localhost:11434".to_string(),
            "nomic-embed-text".to_string(),
            768,
        );

        assert_eq!(generator.embedding_dimension(), 768);
        assert_eq!(generator.model, "nomic-embed-text");
    }

    #[test]
    fn test_from_config() {
        let generator = OllamaEmbeddingGenerator::from_config().unwrap();
        assert_eq!(generator.embedding_dimension(), 768);
        assert_eq!(generator.base_url, "http://localhost:11434");
    }
}
