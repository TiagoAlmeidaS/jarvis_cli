//! LLM processor — generic OpenAI-compatible client for pipeline use.
//!
//! Supports any provider that exposes a `/chat/completions` endpoint:
//! - OpenRouter (recommended for low-cost / free models)
//! - Google AI Studio (Gemini)
//! - OpenAI
//! - Ollama (local)
//! - Any OpenAI-compatible API

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{DaemonPipeline, LlmResponse};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use tracing::{debug, warn};

/// Generic LLM client trait used by pipelines.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate text from a prompt.
    async fn generate(&self, prompt: &str, system: Option<&str>) -> Result<LlmResponse>;

    /// Generate text and parse the response as structured JSON.
    async fn generate_json(&self, prompt: &str, system: Option<&str>) -> Result<serde_json::Value> {
        let response = self.generate(prompt, system).await?;
        let json_text = extract_json_from_response(&response.text);
        let parsed: serde_json::Value = serde_json::from_str(&json_text)?;
        Ok(parsed)
    }
}

/// Helper to call generate_json and deserialize into a concrete type.
pub async fn generate_structured<T: DeserializeOwned>(
    client: &dyn LlmClient,
    prompt: &str,
    system: Option<&str>,
) -> Result<T> {
    let value = client.generate_json(prompt, system).await?;
    let parsed: T = serde_json::from_value(value)?;
    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Provider definitions
// ---------------------------------------------------------------------------

/// Known LLM providers with their default base URLs and env var names.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    /// OpenRouter — aggregator with many cheap/free models.
    #[default]
    Openrouter,
    /// Google AI Studio — Gemini models (has free tier).
    Google,
    /// OpenAI — GPT models.
    Openai,
    /// Ollama — local models.
    Ollama,
    /// Databricks — enterprise models.
    Databricks,
    /// Any custom OpenAI-compatible endpoint.
    Custom,
}

impl LlmProvider {
    /// Default base URL for this provider.
    fn default_base_url(&self) -> &str {
        match self {
            Self::Openrouter => "https://openrouter.ai/api/v1",
            Self::Google => "https://generativelanguage.googleapis.com/v1beta/openai",
            Self::Openai => "https://api.openai.com/v1",
            Self::Ollama => "http://localhost:11434/v1",
            Self::Databricks => "",
            Self::Custom => "",
        }
    }

    /// Environment variable names to check for the API key (in order).
    fn env_key_names(&self) -> &[&str] {
        match self {
            Self::Openrouter => &["OPENROUTER_API_KEY"],
            Self::Google => &["GOOGLE_API_KEY", "GEMINI_API_KEY"],
            Self::Openai => &["OPENAI_API_KEY"],
            Self::Ollama => &[], // no auth needed
            Self::Databricks => &["DATABRICKS_API_KEY"],
            Self::Custom => &["DAEMON_LLM_API_KEY"],
        }
    }

    /// Default model for this provider.
    ///
    /// Chosen for best cost-benefit for SEO content generation:
    /// - OpenRouter: mistral-nemo (12B, multilingual, $0.02-0.04/M tokens)
    /// - Google: gemini-2.0-flash (free tier available)
    /// - Ollama: llama3.2 (local, zero cost)
    fn default_model(&self) -> &str {
        match self {
            Self::Openrouter => "mistralai/mistral-nemo",
            Self::Google => "gemini-2.0-flash",
            Self::Openai => "gpt-4o-mini",
            Self::Ollama => "llama3.2",
            Self::Databricks => "databricks-claude-haiku-4-5",
            Self::Custom => "default",
        }
    }

    /// Approximate blended cost per token (input+output average) for spend estimation.
    ///
    /// Pricing source: OpenRouter API /models endpoint (2026-02).
    /// For exact per-model pricing, see <https://openrouter.ai/models>.
    fn cost_per_token(self, model: &str) -> f64 {
        match self {
            Self::Openrouter => openrouter_cost_per_token(model),
            Self::Google => 0.000000075, // Gemini Flash ~$0.075/M
            Self::Openai => 0.000002,    // gpt-4o-mini ~$2/M blended
            Self::Ollama => 0.0,         // local, no cost
            Self::Databricks => 0.000001,
            Self::Custom => 0.0,
        }
    }
}

/// Blended cost per token for known OpenRouter models.
///
/// Returns an average of input and output cost weighted for typical SEO
/// content generation (~40% input, ~60% output tokens).
///
/// Pricing data from OpenRouter API (2026-02). Models not listed get a
/// conservative default estimate.
fn openrouter_cost_per_token(model: &str) -> f64 {
    // Free models.
    if model.contains(":free") || model == "openrouter/free" {
        return 0.0;
    }

    // Known cheap models — blended cost per token (input*0.4 + output*0.6).
    // Pricing: (input_per_token * 0.4 + output_per_token * 0.6)
    match model {
        // Tier 1: Ultra-cheap ($0.02-0.10/M)
        "meta-llama/llama-3.2-3b-instruct" => 0.00000002, // $0.02/M in+out
        "mistralai/mistral-nemo" => 0.000000032,          // $0.02 in, $0.04 out
        "meta-llama/llama-3.1-8b-instruct" => 0.000000038, // $0.02 in, $0.05 out
        "meta-llama/llama-3-8b-instruct" => 0.000000036,  // $0.03 in, $0.04 out
        "nousresearch/deephermes-3-mistral-24b-preview" => 0.000000068, // $0.02 in, $0.10 out

        // Tier 2: Very cheap ($0.06-0.40/M)
        "z-ai/glm-4.7-flash" => 0.000000264, // $0.06 in, $0.40 out
        "qwen/qwen3-coder-next" => 0.000000208, // $0.07 in, $0.30 out
        "bytedance-seed/seed-1.6-flash" => 0.00000021, // $0.075 in, $0.30 out
        "stepfun/step-3.5-flash" => 0.00000022, // $0.10 in, $0.30 out
        "allenai/olmo-3.1-32b-instruct" => 0.00000044, // $0.20 in, $0.60 out

        // Tier 3: Mid-range ($0.30-1.20/M)
        "minimax/minimax-m2.5" => 0.00000084, // $0.30 in, $1.20 out
        "moonshotai/kimi-k2.5" => 0.00000153, // $0.45 in, $2.25 out
        "google/gemini-2.5-flash" => 0.00000018, // $0.15 in, $0.60 out (if routed)

        // Default for unknown OpenRouter models.
        _ => 0.0000005, // ~$0.50/M conservative estimate
    }
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Openrouter => write!(f, "openrouter"),
            Self::Google => write!(f, "google"),
            Self::Openai => write!(f, "openai"),
            Self::Ollama => write!(f, "ollama"),
            Self::Databricks => write!(f, "databricks"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for LlmProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "openrouter" => Ok(Self::Openrouter),
            "google" => Ok(Self::Google),
            "openai" => Ok(Self::Openai),
            "ollama" => Ok(Self::Ollama),
            "databricks" => Ok(Self::Databricks),
            "custom" => Ok(Self::Custom),
            other => Err(anyhow::anyhow!("unknown LLM provider: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// LLM configuration
// ---------------------------------------------------------------------------

/// LLM configuration extracted from pipeline config JSON.
///
/// Example configs:
///
/// ```json
/// // OpenRouter free (zero cost)
/// { "provider": "openrouter", "model": "openrouter/free" }
///
/// // OpenRouter cheap
/// { "provider": "openrouter", "model": "qwen/qwen3-coder-next" }
///
/// // Google Gemini free tier
/// { "provider": "google", "model": "gemini-2.0-flash" }
///
/// // Local Ollama
/// { "provider": "ollama", "model": "llama3.2", "base_url": "http://100.98.213.86:11434/v1" }
///
/// // Custom endpoint
/// { "provider": "custom", "base_url": "https://my-api.com/v1", "api_key": "sk-..." }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    /// Provider name (openrouter, google, openai, ollama, databricks, custom).
    #[serde(default)]
    pub provider: LlmProvider,
    /// Model identifier. Defaults vary by provider.
    pub model: Option<String>,
    /// Override the base URL (useful for custom endpoints or remote Ollama).
    pub base_url: Option<String>,
    /// Explicit API key (not recommended; prefer env vars).
    pub api_key: Option<String>,
    /// Max tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Sampling temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Extra HTTP headers to send (e.g., for OpenRouter site identification).
    #[serde(default)]
    pub extra_headers: std::collections::HashMap<String, String>,
}

fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.7
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::default(),
            model: None,
            base_url: None,
            api_key: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            extra_headers: std::collections::HashMap::new(),
        }
    }
}

impl LlmConfig {
    /// Resolve the effective model name.
    pub fn effective_model(&self) -> String {
        self.model
            .clone()
            .unwrap_or_else(|| self.provider.default_model().to_string())
    }

    /// Resolve the effective base URL.
    pub fn effective_base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| self.provider.default_base_url().to_string())
    }

    /// Resolve the API key from config or environment variables.
    pub fn resolve_api_key(&self) -> Result<String> {
        // Explicit key in config takes priority.
        if let Some(ref key) = self.api_key {
            return Ok(key.clone());
        }

        // Try each env var name for this provider.
        for env_name in self.provider.env_key_names() {
            if let Ok(key) = std::env::var(env_name)
                && !key.is_empty()
            {
                return Ok(key);
            }
        }

        // Ollama doesn't need a key.
        if self.provider == LlmProvider::Ollama {
            return Ok(String::new());
        }

        let env_names = self
            .provider
            .env_key_names()
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>()
            .join(" or ");

        Err(anyhow::anyhow!(
            "No API key found for provider '{}'. Set {} environment variable.",
            self.provider,
            env_names,
        ))
    }
}

// ---------------------------------------------------------------------------
// Generic OpenAI-compatible LLM client
// ---------------------------------------------------------------------------

/// A generic LLM client that works with any OpenAI-compatible API.
pub struct OpenAiCompatibleClient {
    http: reqwest::Client,
    config: LlmConfig,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAiCompatibleClient {
    /// Create a client from a pipeline's LLM configuration.
    pub fn from_pipeline_config(pipeline: &DaemonPipeline) -> Result<Self> {
        let config_value: serde_json::Value = serde_json::from_str(&pipeline.config_json)?;
        let llm_config: LlmConfig = config_value
            .get("llm")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or_default();

        Self::from_config(llm_config)
    }

    /// Create a client from an explicit config.
    pub fn from_config(config: LlmConfig) -> Result<Self> {
        let api_key = config.resolve_api_key()?;
        let base_url = config.effective_base_url();
        let model = config.effective_model();

        debug!(
            "LLM client initialized: provider={}, model={model}, base_url={base_url}",
            config.provider
        );

        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()?,
            config,
            api_key,
            base_url,
            model,
        })
    }

    /// Create with explicit parameters (for testing).
    #[cfg(test)]
    pub fn new_test(base_url: String, api_key: String, model: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            config: LlmConfig::default(),
            api_key,
            base_url,
            model,
        }
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn generate(&self, prompt: &str, system: Option<&str>) -> Result<LlmResponse> {
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        let url = format!("{}/chat/completions", self.base_url);

        let mut request = self.http.post(&url).json(&body);

        // Add auth header (skip for Ollama or empty keys).
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }

        // Add extra headers (e.g., OpenRouter requires HTTP-Referer for ranking).
        for (key, value) in &self.config.extra_headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // OpenRouter recommends identifying your app.
        if self.config.provider == LlmProvider::Openrouter {
            request = request
                .header("HTTP-Referer", "https://github.com/jarvis-cli")
                .header("X-Title", "Jarvis Daemon");
        }

        let resp = request.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "LLM API error {status} ({}, {}):\n{text}",
                self.config.provider,
                self.model
            );
        }

        let json: serde_json::Value = resp.json().await?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tokens_used = json["usage"]["total_tokens"].as_i64().unwrap_or(0);

        // Estimate cost based on provider/model.
        let cost_per_token = self.config.provider.cost_per_token(&self.model);
        let cost_usd = if tokens_used > 0 && cost_per_token > 0.0 {
            Some(tokens_used as f64 * cost_per_token)
        } else {
            None
        };

        if text.is_empty() {
            warn!(
                "LLM returned empty response (provider={}, model={})",
                self.config.provider, self.model
            );
        }

        Ok(LlmResponse {
            text,
            model: self.model.clone(),
            tokens_used,
            cost_usd,
        })
    }
}

/// Extract JSON from a response that may be wrapped in markdown code blocks.
fn extract_json_from_response(text: &str) -> String {
    let trimmed = text.trim();

    // Try to find ```json ... ``` block.
    if let Some(start) = trimmed.find("```json") {
        let after_marker = &trimmed[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }

    // Try to find ``` ... ``` block.
    if let Some(start) = trimmed.find("```") {
        let after_marker = &trimmed[start + 3..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }

    // Try to find raw JSON object.
    if let Some(start) = trimmed.find('{')
        && let Some(end) = trimmed.rfind('}')
    {
        return trimmed[start..=end].to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn extract_json_from_code_block() {
        let input = r#"Here's the result:
```json
{"title": "Hello World", "content": "Some text"}
```
Hope this helps!"#;

        let result = extract_json_from_response(input);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap_or_default();
        assert_eq!(parsed["title"], "Hello World");
    }

    #[test]
    fn extract_json_from_raw() {
        let input = r#"{"title": "Direct JSON"}"#;
        let result = extract_json_from_response(input);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap_or_default();
        assert_eq!(parsed["title"], "Direct JSON");
    }

    #[test]
    fn extract_json_with_surrounding_text() {
        let input = r#"Sure, here's the output: {"title": "Nested"} as requested."#;
        let result = extract_json_from_response(input);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap_or_default();
        assert_eq!(parsed["title"], "Nested");
    }

    #[test]
    fn default_provider_is_openrouter() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::Openrouter);
        assert_eq!(config.effective_model(), "mistralai/mistral-nemo");
        assert_eq!(config.effective_base_url(), "https://openrouter.ai/api/v1");
    }

    #[test]
    fn provider_round_trip() {
        for name in &[
            "openrouter",
            "google",
            "openai",
            "ollama",
            "databricks",
            "custom",
        ] {
            let p: LlmProvider = name.parse().unwrap_or_default();
            assert_eq!(p.to_string(), *name);
        }
    }

    #[test]
    fn config_deserialize_openrouter() {
        let json = serde_json::json!({
            "provider": "openrouter",
            "model": "qwen/qwen3-coder-next"
        });
        let config: LlmConfig = serde_json::from_value(json).unwrap_or_default();
        assert_eq!(config.provider, LlmProvider::Openrouter);
        assert_eq!(config.effective_model(), "qwen/qwen3-coder-next");
        assert_eq!(config.effective_base_url(), "https://openrouter.ai/api/v1");
    }

    #[test]
    fn config_deserialize_google() {
        let json = serde_json::json!({
            "provider": "google",
            "model": "gemini-2.0-flash"
        });
        let config: LlmConfig = serde_json::from_value(json).unwrap_or_default();
        assert_eq!(config.provider, LlmProvider::Google);
        assert_eq!(config.effective_model(), "gemini-2.0-flash");
    }

    #[test]
    fn config_deserialize_ollama_with_custom_url() {
        let json = serde_json::json!({
            "provider": "ollama",
            "model": "llama3.2",
            "base_url": "http://100.98.213.86:11434/v1"
        });
        let config: LlmConfig = serde_json::from_value(json).unwrap_or_default();
        assert_eq!(config.provider, LlmProvider::Ollama);
        assert_eq!(config.effective_base_url(), "http://100.98.213.86:11434/v1");
    }

    #[test]
    fn config_deserialize_custom_with_key() {
        let json = serde_json::json!({
            "provider": "custom",
            "base_url": "https://my-proxy.com/v1",
            "api_key": "sk-test-123",
            "model": "my-model"
        });
        let config: LlmConfig = serde_json::from_value(json).unwrap_or_default();
        assert_eq!(config.provider, LlmProvider::Custom);
        let key = config.resolve_api_key().unwrap_or_default();
        assert_eq!(key, "sk-test-123");
    }

    #[test]
    fn ollama_doesnt_need_api_key() {
        let config = LlmConfig {
            provider: LlmProvider::Ollama,
            ..Default::default()
        };
        let key = config.resolve_api_key().unwrap_or_default();
        assert!(key.is_empty());
    }

    #[test]
    fn free_model_cost_is_zero() {
        let cost = LlmProvider::Openrouter.cost_per_token("openrouter/free");
        assert_eq!(cost, 0.0);

        let cost = LlmProvider::Openrouter.cost_per_token("meta-llama/llama-3.3-8b-instruct:free");
        assert_eq!(cost, 0.0);

        let cost = LlmProvider::Ollama.cost_per_token("llama3.2");
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn known_cheap_models_have_specific_costs() {
        // Tier 1 ultra-cheap models should have very low costs.
        let nemo = LlmProvider::Openrouter.cost_per_token("mistralai/mistral-nemo");
        assert!(nemo > 0.0 && nemo < 0.0000001, "mistral-nemo cost: {nemo}");

        let llama8b = LlmProvider::Openrouter.cost_per_token("meta-llama/llama-3.1-8b-instruct");
        assert!(
            llama8b > 0.0 && llama8b < 0.0000001,
            "llama-3.1-8b cost: {llama8b}"
        );

        // Tier 2 models should cost more but still be very cheap.
        let qwen = LlmProvider::Openrouter.cost_per_token("qwen/qwen3-coder-next");
        assert!(qwen > nemo, "qwen should cost more than mistral-nemo");
        assert!(qwen < 0.000001, "qwen should still be very cheap: {qwen}");

        // Unknown model gets conservative default.
        let unknown = LlmProvider::Openrouter.cost_per_token("some/unknown-model");
        assert!(
            unknown > qwen,
            "unknown model should get conservative estimate"
        );
    }
}
