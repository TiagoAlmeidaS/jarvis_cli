//! Utilities for fetching free models from OpenRouter API.

use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::RwLock;

/// Cache entry for OpenRouter free models.
#[derive(Debug, Clone)]
struct CacheEntry {
    models: Vec<String>,
    fetched_at: Instant,
}

/// Cache TTL: 5 minutes
const CACHE_TTL: Duration = Duration::from_secs(300);

static CACHE: OnceLock<Arc<RwLock<Option<CacheEntry>>>> = OnceLock::new();

fn get_cache() -> Arc<RwLock<Option<CacheEntry>>> {
    CACHE.get_or_init(|| Arc::new(RwLock::new(None))).clone()
}

/// Model data from OpenRouter API.
#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    #[serde(default)]
    pricing: Option<ModelPricing>,
}

/// Pricing information for a model.
#[derive(Debug, Deserialize)]
struct ModelPricing {
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    completion: Option<String>,
}

/// Response from OpenRouter /models endpoint.
#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

/// Check if a model is free based on its pricing.
fn is_free_model(model: &OpenRouterModel) -> bool {
    // A model is free if:
    // 1. It has no pricing info, OR
    // 2. Both prompt and completion pricing are "0" or null
    match &model.pricing {
        None => true,
        Some(pricing) => {
            let prompt_free = pricing
                .prompt
                .as_ref()
                .map(|p| p == "0" || p.trim().is_empty())
                .unwrap_or(true);
            let completion_free = pricing
                .completion
                .as_ref()
                .map(|c| c == "0" || c.trim().is_empty())
                .unwrap_or(true);
            prompt_free && completion_free
        }
    }
}

/// Fetch free models from OpenRouter API.
///
/// Returns a list of model IDs that are free, or None if the request fails.
/// Results are cached for 5 minutes to avoid excessive API calls.
pub async fn fetch_free_models(api_key: Option<&str>) -> Option<Vec<String>> {
    // Check cache first
    let cache = get_cache();
    {
        let cached = cache.read().await;
        if let Some(entry) = cached.as_ref() {
            if entry.fetched_at.elapsed() < CACHE_TTL {
                return Some(entry.models.clone());
            }
        }
    }

    // Fetch from API
    let client = crate::default_client::build_reqwest_client();
    let mut request = client
        .get("https://openrouter.ai/api/v1/models")
        .header("HTTP-Referer", "https://github.com/jarvis-cli")
        .header("X-Title", "Jarvis CLI");

    // Add API key if provided
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }

    let response = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::debug!("Failed to fetch OpenRouter models: {}", e);
            return None;
        }
    };

    if !response.status().is_success() {
        tracing::debug!(
            "OpenRouter models API returned error: {}",
            response.status()
        );
        return None;
    }

    let models_response: OpenRouterModelsResponse = match response.json().await {
        Ok(data) => data,
        Err(e) => {
            tracing::debug!("Failed to parse OpenRouter models response: {}", e);
            return None;
        }
    };

    // Filter free models
    let free_models: Vec<String> = models_response
        .data
        .into_iter()
        .filter(is_free_model)
        .map(|model| model.id)
        .collect();

    // Update cache
    {
        let mut cache_write = cache.write().await;
        *cache_write = Some(CacheEntry {
            models: free_models.clone(),
            fetched_at: Instant::now(),
        });
    }

    Some(free_models)
}

/// Get a limited list of free models (up to max_count) for error messages.
pub async fn get_free_models_for_error(
    api_key: Option<&str>,
    max_count: usize,
) -> Option<Vec<String>> {
    fetch_free_models(api_key)
        .await
        .map(|models| models.into_iter().take(max_count).collect())
}
