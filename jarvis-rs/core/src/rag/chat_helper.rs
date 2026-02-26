//! Helper functions for integrating RAG with chat.
//!
//! This module provides utilities to inject RAG context into chat messages
//! before they are sent to the LLM.

use crate::rag::chat_integration::RagContextConfig;
use crate::rag::chat_integration::RagContextInjector;
use anyhow::Result;
use std::sync::Arc;

/// Prepares a user message by injecting relevant RAG context.
///
/// This function:
/// 1. Takes the user's original message
/// 2. Searches for relevant context in the RAG system
/// 3. Injects the context into the message (if any is found)
/// 4. Returns the enhanced message ready for the LLM
///
/// # Arguments
/// * `user_message` - The original user message
/// * `injector` - The RAG context injector
/// * `config` - Configuration for RAG retrieval
///
/// # Returns
/// The enhanced message with RAG context prepended (if relevant context was found),
/// otherwise returns the original message.
///
/// # Example
/// ```no_run
/// use jarvis_core::rag::chat_helper::inject_rag_context;
/// use jarvis_core::rag::{RagContextInjector, RagContextConfig};
/// use std::sync::Arc;
///
/// async fn process_message(injector: Arc<RagContextInjector>, message: String) -> String {
///     let config = RagContextConfig::default();
///     inject_rag_context(&message, &injector, &config).await.unwrap_or(message)
/// }
/// ```
pub async fn inject_rag_context(
    user_message: &str,
    injector: &RagContextInjector,
    config: &RagContextConfig,
) -> Result<String> {
    if !config.enabled || !injector.is_enabled() {
        return Ok(user_message.to_string());
    }

    // Get relevant context for the user's query
    let context = injector.get_relevant_context(user_message, config).await?;

    // If no context was found, return original message
    if context.is_empty() {
        return Ok(user_message.to_string());
    }

    // Prepend context to the user message
    let enhanced_message = format!("{}\n\n{}", context, user_message);

    Ok(enhanced_message)
}

/// Creates a RAG context injector from a [`RagConfig`].
///
/// This function attempts to create a RAG injector using the supplied
/// configuration. If the config has `enabled: false` or setup fails,
/// returns a disabled injector.
pub async fn create_rag_injector_from_config(
    cfg: &crate::config::types::RagConfig,
) -> Arc<RagContextInjector> {
    if !cfg.enabled {
        return Arc::new(create_disabled_injector());
    }

    match RagContextInjector::from_rag_config(cfg).await {
        Ok(injector) => Arc::new(injector),
        Err(e) => {
            tracing::warn!("Failed to initialize RAG context injector: {e}. RAG will be disabled.",);
            Arc::new(create_disabled_injector())
        }
    }
}

/// Creates a RAG context injector with smart fallback (legacy).
///
/// This function attempts to create a RAG injector with the following priority:
/// 1. Full setup: Qdrant + PostgreSQL + Ollama
/// 2. Fallback: In-memory vector store + JSON file document store + Ollama
/// 3. Disabled: If Ollama is not available
///
/// # Returns
/// An Arc-wrapped RagContextInjector that can be cloned and used across threads.
///
/// # Example
/// ```no_run
/// use jarvis_core::rag::chat_helper::create_rag_injector;
///
/// #[tokio::main]
/// async fn main() {
///     let injector = create_rag_injector().await;
///     println!("RAG enabled: {}", injector.is_enabled());
/// }
/// ```
pub async fn create_rag_injector() -> Arc<RagContextInjector> {
    match RagContextInjector::from_config().await {
        Ok(injector) => Arc::new(injector),
        Err(e) => {
            tracing::warn!(
                "Failed to initialize RAG context injector: {}. RAG will be disabled.",
                e
            );
            // Create a disabled injector as fallback
            Arc::new(create_disabled_injector())
        }
    }
}

/// Creates a disabled RAG injector (for when RAG setup fails).
fn create_disabled_injector() -> RagContextInjector {
    create_disabled_injector_with_config(None)
}

/// Creates a disabled RAG injector, optionally using [`RagConfig`] defaults.
///
/// If a config is provided the Ollama embedding generator will use its URL,
/// model and dimension settings instead of hardcoded localhost defaults.
pub fn create_disabled_injector_with_config(
    cfg: Option<&crate::config::types::RagConfig>,
) -> RagContextInjector {
    use crate::rag::InMemoryDocumentStore;
    use crate::rag::InMemoryVectorStore;
    use crate::rag::OllamaEmbeddingGenerator;
    use std::sync::Arc;

    let embedding_gen = Arc::new(match cfg {
        Some(c) => OllamaEmbeddingGenerator::from_rag_config(c),
        None => OllamaEmbeddingGenerator::from_config().unwrap_or_else(|_| {
            OllamaEmbeddingGenerator::new(
                "http://localhost:11434".to_string(),
                "nomic-embed-text".to_string(),
                768,
            )
        }),
    });
    let vector_store = Arc::new(InMemoryVectorStore::new());
    let doc_store = Arc::new(InMemoryDocumentStore::new());

    // Create injector but disable it
    let mut injector = RagContextInjector::new(
        embedding_gen as Arc<dyn crate::rag::EmbeddingGenerator>,
        vector_store as Arc<dyn crate::rag::VectorStore>,
        doc_store as Arc<dyn crate::rag::DocumentStore>,
        false, // disabled
    );
    injector.set_enabled(false);
    injector
}

/// Checks if RAG is available and ready to use.
///
/// This is a lightweight check that verifies:
/// 1. The injector is enabled
/// 2. Ollama embedding service is reachable
/// 3. Vector store is available
///
/// # Example
/// ```no_run
/// use jarvis_core::rag::chat_helper::{create_rag_injector, is_rag_ready};
///
/// #[tokio::main]
/// async fn main() {
///     let injector = create_rag_injector().await;
///     if is_rag_ready(&injector).await {
///         println!("RAG is ready!");
///     }
/// }
/// ```
pub async fn is_rag_ready(injector: &RagContextInjector) -> bool {
    if !injector.is_enabled() {
        return false;
    }

    // Try to get stats to verify everything is working
    injector.get_context_stats().await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inject_rag_context_disabled() {
        let injector = create_disabled_injector();
        let config = RagContextConfig {
            enabled: false,
            ..Default::default()
        };

        let message = "Test message";
        let result = inject_rag_context(message, &injector, &config)
            .await
            .unwrap();

        assert_eq!(
            result, message,
            "Message should be unchanged when RAG is disabled"
        );
    }

    #[tokio::test]
    async fn test_create_disabled_injector() {
        let injector = create_disabled_injector();
        assert!(
            !injector.is_enabled(),
            "Disabled injector should not be enabled"
        );
    }

    #[tokio::test]
    async fn test_is_rag_ready_for_disabled() {
        let injector = create_disabled_injector();
        assert!(
            !is_rag_ready(&injector).await,
            "Disabled injector should not be ready"
        );
    }
}
