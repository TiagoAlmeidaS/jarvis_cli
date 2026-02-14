//! Registry of model providers supported by Jarvis.
//!
//! Providers can be defined in two places:
//!   1. Built-in defaults compiled into the binary so Jarvis works out-of-the-box.
//!   2. User-defined entries inside `~/.jarvis/config.toml` under the `model_providers`
//!      key. These override or extend the defaults at runtime.

use crate::auth::AuthMode;
use crate::error::EnvVarError;
use http::HeaderMap;
use http::header::HeaderName;
use http::header::HeaderValue;
use jarvis_api::Provider as ApiProvider;
use jarvis_api::is_azure_responses_wire_base_url;
use jarvis_api::provider::RetryConfig as ApiRetryConfig;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::env::VarError;
use std::time::Duration;

const DEFAULT_STREAM_IDLE_TIMEOUT_MS: u64 = 300_000;
const DEFAULT_STREAM_MAX_RETRIES: u64 = 5;
const DEFAULT_REQUEST_MAX_RETRIES: u64 = 4;
/// Hard cap for user-configured `stream_max_retries`.
const MAX_STREAM_MAX_RETRIES: u64 = 100;
/// Hard cap for user-configured `request_max_retries`.
const MAX_REQUEST_MAX_RETRIES: u64 = 100;

const OPENAI_PROVIDER_NAME: &str = "OpenAI";
const AZURE_OPENAI_PROVIDER_NAME: &str = "Azure OpenAI";
const DEFAULT_AZURE_OPENAI_API_VERSION: &str = "2024-08-01-preview";
const CHAT_WIRE_API_REMOVED_ERROR: &str = "`wire_api = \"chat\"` is no longer supported.\nHow to fix: set `wire_api = \"responses\"` in your provider config.\nMore info: https://github.com/openai/Jarvis/discussions/7782";
pub(crate) const LEGACY_OLLAMA_CHAT_PROVIDER_ID: &str = "ollama-chat";
pub(crate) const OLLAMA_CHAT_PROVIDER_REMOVED_ERROR: &str = "`ollama-chat` is no longer supported.\nHow to fix: replace `ollama-chat` with `ollama` in `model_provider`, `oss_provider`, or `--local-provider`.\nMore info: https://github.com/openai/Jarvis/discussions/7782";

/// Wire protocol that the provider speaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WireApi {
    /// The Responses API exposed by OpenAI at `/v1/responses`.
    #[default]
    Responses,
}

impl<'de> Deserialize<'de> for WireApi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "responses" => Ok(Self::Responses),
            "chat" => Err(serde::de::Error::custom(CHAT_WIRE_API_REMOVED_ERROR)),
            _ => Err(serde::de::Error::unknown_variant(&value, &["responses"])),
        }
    }
}

/// Serializable representation of a provider definition.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ModelProviderInfo {
    /// Friendly display name.
    pub name: String,
    /// Base URL for the provider's OpenAI-compatible API.
    pub base_url: Option<String>,
    /// Environment variable that stores the user's API key for this provider.
    pub env_key: Option<String>,

    /// Optional instructions to help the user get a valid value for the
    /// variable and set it.
    pub env_key_instructions: Option<String>,

    /// Value to use with `Authorization: Bearer <token>` header. Use of this
    /// config is discouraged in favor of `env_key` for security reasons, but
    /// this may be necessary when using this programmatically.
    pub experimental_bearer_token: Option<String>,

    /// Which wire protocol this provider expects.
    #[serde(default)]
    pub wire_api: WireApi,

    /// Optional query parameters to append to the base URL.
    pub query_params: Option<HashMap<String, String>>,

    /// Additional HTTP headers to include in requests to this provider where
    /// the (key, value) pairs are the header name and value.
    pub http_headers: Option<HashMap<String, String>>,

    /// Optional HTTP headers to include in requests to this provider where the
    /// (key, value) pairs are the header name and _environment variable_ whose
    /// value should be used. If the environment variable is not set, or the
    /// value is empty, the header will not be included in the request.
    pub env_http_headers: Option<HashMap<String, String>>,

    /// Maximum number of times to retry a failed HTTP request to this provider.
    pub request_max_retries: Option<u64>,

    /// Number of times to retry reconnecting a dropped streaming response before failing.
    pub stream_max_retries: Option<u64>,

    /// Idle timeout (in milliseconds) to wait for activity on a streaming response before treating
    /// the connection as lost.
    pub stream_idle_timeout_ms: Option<u64>,

    /// Does this provider require an OpenAI API Key or ChatGPT login token? If true,
    /// user is presented with login screen on first run, and login preference and token/key
    /// are stored in auth.json. If false (which is the default), login screen is skipped,
    /// and API key (if needed) comes from the "env_key" environment variable.
    #[serde(default)]
    pub requires_openai_auth: bool,

    /// Whether this provider supports the Responses API WebSocket transport.
    #[serde(default)]
    pub supports_websockets: bool,

    /// If true, this provider uses Chat Completions API (`/v1/chat/completions`)
    /// instead of Responses API (`/v1/responses`). When enabled, requests will be
    /// automatically converted from Responses format to Chat Completions format.
    /// This is used for providers like Ollama and others that implement the
    /// OpenAI Chat Completions API but not the Responses API.
    #[serde(default)]
    pub uses_chat_completions_api: bool,
}

impl ModelProviderInfo {
    fn build_header_map(&self) -> crate::error::Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        if let Some(extra) = &self.http_headers {
            for (k, v) in extra {
                if let (Ok(name), Ok(value)) = (HeaderName::try_from(k), HeaderValue::try_from(v)) {
                    headers.insert(name, value);
                }
            }
        }

        if let Some(env_headers) = &self.env_http_headers {
            for (header, env_var) in env_headers {
                if let Ok(val) = std::env::var(env_var)
                    && !val.trim().is_empty()
                    && let (Ok(name), Ok(value)) =
                        (HeaderName::try_from(header), HeaderValue::try_from(val))
                {
                    headers.insert(name, value);
                }
            }
        }

        Ok(headers)
    }

    pub(crate) fn to_api_provider(
        &self,
        auth_mode: Option<AuthMode>,
    ) -> crate::error::Result<ApiProvider> {
        let default_base_url = if matches!(auth_mode, Some(AuthMode::Chatgpt)) {
            "https://chatgpt.com/backend-api/Jarvis"
        } else {
            "https://api.openai.com/v1"
        };
        let base_url = self
            .base_url
            .clone()
            .unwrap_or_else(|| default_base_url.to_string());

        let headers = self.build_header_map()?;
        let retry = ApiRetryConfig {
            max_attempts: self.request_max_retries(),
            base_delay: Duration::from_millis(200),
            retry_429: false,
            retry_5xx: true,
            retry_transport: true,
        };

        Ok(ApiProvider {
            name: self.name.clone(),
            base_url,
            query_params: self.query_params.clone(),
            headers,
            retry,
            stream_idle_timeout: self.stream_idle_timeout(),
            uses_chat_completions_api: self.uses_chat_completions_api,
        })
    }

    pub(crate) fn is_azure_responses_endpoint(&self) -> bool {
        is_azure_responses_wire_base_url(&self.name, self.base_url.as_deref())
    }

    /// If `env_key` is Some, returns the API key for this provider if present
    /// (and non-empty) in the environment. If `env_key` is required but
    /// cannot be found, returns an error.
    pub fn api_key(&self) -> crate::error::Result<Option<String>> {
        match &self.env_key {
            Some(env_key) => {
                let env_value = std::env::var(env_key);
                env_value
                    .and_then(|v| {
                        if v.trim().is_empty() {
                            Err(VarError::NotPresent)
                        } else {
                            Ok(Some(v))
                        }
                    })
                    .map_err(|_| {
                        crate::error::JarvisErr::EnvVar(EnvVarError {
                            var: env_key.clone(),
                            instructions: self.env_key_instructions.clone(),
                        })
                    })
            }
            None => Ok(None),
        }
    }

    /// Effective maximum number of request retries for this provider.
    pub fn request_max_retries(&self) -> u64 {
        self.request_max_retries
            .unwrap_or(DEFAULT_REQUEST_MAX_RETRIES)
            .min(MAX_REQUEST_MAX_RETRIES)
    }

    /// Effective maximum number of stream reconnection attempts for this provider.
    pub fn stream_max_retries(&self) -> u64 {
        self.stream_max_retries
            .unwrap_or(DEFAULT_STREAM_MAX_RETRIES)
            .min(MAX_STREAM_MAX_RETRIES)
    }

    /// Effective idle timeout for streaming responses.
    pub fn stream_idle_timeout(&self) -> Duration {
        self.stream_idle_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_STREAM_IDLE_TIMEOUT_MS))
    }
    pub fn create_openai_provider() -> ModelProviderInfo {
        ModelProviderInfo {
            name: OPENAI_PROVIDER_NAME.into(),
            // Allow users to override the default OpenAI endpoint by
            // exporting `OPENAI_BASE_URL`. This is useful when pointing
            // Jarvis at a proxy, mock server, or Azure-style deployment
            // without requiring a full TOML override for the built-in
            // OpenAI provider.
            base_url: std::env::var("OPENAI_BASE_URL")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: Some(
                [("version".to_string(), env!("CARGO_PKG_VERSION").to_string())]
                    .into_iter()
                    .collect(),
            ),
            env_http_headers: Some(
                [
                    (
                        "OpenAI-Organization".to_string(),
                        "OPENAI_ORGANIZATION".to_string(),
                    ),
                    ("OpenAI-Project".to_string(), "OPENAI_PROJECT".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
            // Use global defaults for retry/timeout unless overridden in config.toml.
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            requires_openai_auth: true,
            supports_websockets: true,
            uses_chat_completions_api: false,
        }
    }

    pub fn is_openai(&self) -> bool {
        self.name == OPENAI_PROVIDER_NAME
    }
}

pub const DEFAULT_LMSTUDIO_PORT: u16 = 1234;
pub const DEFAULT_OLLAMA_PORT: u16 = 11434;

pub const LMSTUDIO_OSS_PROVIDER_ID: &str = "lmstudio";
pub const OLLAMA_OSS_PROVIDER_ID: &str = "ollama";
pub const OPENROUTER_PROVIDER_ID: &str = "openrouter";
pub const GOOGLE_PROVIDER_ID: &str = "google";

/// Built-in default provider list.
pub fn built_in_model_providers() -> HashMap<String, ModelProviderInfo> {
    use ModelProviderInfo as P;

    // Databricks is prioritized as the default provider for LLM requests.
    // Users can add additional providers in config.toml as needed.
    [
        ("databricks", create_databricks_provider(None)),
        ("openai", P::create_openai_provider()),
        (
            OLLAMA_OSS_PROVIDER_ID,
            create_ollama_provider(None), // Use dedicated Ollama provider
        ),
        (
            LMSTUDIO_OSS_PROVIDER_ID,
            create_oss_provider(DEFAULT_LMSTUDIO_PORT, WireApi::Responses),
        ),
        (OPENROUTER_PROVIDER_ID, create_openrouter_provider()),
        (GOOGLE_PROVIDER_ID, create_google_provider()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

pub fn create_oss_provider(default_provider_port: u16, wire_api: WireApi) -> ModelProviderInfo {
    // These jarvis_OSS_ environment variables are experimental: we may
    // switch to reading values from config.toml instead.
    let jarvis_oss_base_url = match std::env::var("jarvis_OSS_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        Some(url) => url,
        None => format!(
            "http://localhost:{port}/v1",
            port = std::env::var("jarvis_OSS_PORT")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(default_provider_port)
        ),
    };
    create_oss_provider_with_base_url(&jarvis_oss_base_url, wire_api)
}

pub fn create_oss_provider_with_base_url(base_url: &str, wire_api: WireApi) -> ModelProviderInfo {
    ModelProviderInfo {
        name: "gpt-oss".into(),
        base_url: Some(base_url.into()),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        wire_api,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
        uses_chat_completions_api: false,
    }
}

/// Creates an Ollama provider configuration.
///
/// Ollama uses the OpenAI Chat Completions API format. The base URL should be
/// in the format: `http://{host}:{port}/v1` (e.g., `http://localhost:11434/v1`
/// or `http://100.98.213.86:11434/v1` for VPS via Tailscale).
///
/// Ollama does not require authentication by default, but can be configured to
/// use an API key if needed via the `OLLAMA_API_KEY` environment variable.
///
/// Example configuration in `~/.jarvis/config.toml`:
/// ```toml
/// [model_providers.ollama]
/// name = "Ollama"
/// base_url = "http://localhost:11434/v1"
/// uses_chat_completions_api = true  # Automatically set by create_ollama_provider
/// ```
///
/// For VPS via Tailscale:
/// ```toml
/// [model_providers.ollama]
/// name = "Ollama VPS"
/// base_url = "http://100.98.213.86:11434/v1"
/// uses_chat_completions_api = true
/// ```
pub fn create_ollama_provider(base_url: Option<String>) -> ModelProviderInfo {
    // Allow users to override the default Ollama endpoint by
    // exporting `OLLAMA_BASE_URL`. This is useful when pointing
    // Jarvis at a remote Ollama instance via Tailscale or custom setup.
    let effective_base_url = base_url
        .or_else(|| {
            std::env::var("OLLAMA_BASE_URL")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .unwrap_or_else(|| format!("http://localhost:{}/v1", DEFAULT_OLLAMA_PORT));

    ModelProviderInfo {
        name: "Ollama".into(),
        base_url: Some(effective_base_url),
        env_key: None, // Ollama doesn't require auth by default
        env_key_instructions: None,
        experimental_bearer_token: None,
        wire_api: WireApi::Responses, // Internal representation stays Responses
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: Some(DEFAULT_REQUEST_MAX_RETRIES),
        stream_max_retries: Some(DEFAULT_STREAM_MAX_RETRIES),
        stream_idle_timeout_ms: Some(DEFAULT_STREAM_IDLE_TIMEOUT_MS),
        requires_openai_auth: false,
        supports_websockets: false,
        uses_chat_completions_api: true, // Ollama uses Chat Completions API
    }
}

/// Creates a Databricks provider configuration.
///
/// Databricks uses serving endpoints for model inference. The base URL should be
/// in the format: `https://{workspace}.cloud.databricks.com/serving-endpoints/{endpoint}/invocations`
///
/// The API key should be set via the `DATABRICKS_API_KEY` environment variable.
/// The system will automatically construct the `Authorization: Bearer {token}` header
/// from the `env_key` value.
///
/// Example configuration in `~/.jarvis/config.toml`:
/// ```toml
/// [model_providers.databricks]
/// name = "Databricks"
/// base_url = "https://your-workspace.cloud.databricks.com/serving-endpoints/your-endpoint/invocations"
/// env_key = "DATABRICKS_API_KEY"
/// http_headers = { "Content-Type" = "application/json" }
/// ```
pub fn create_databricks_provider(base_url: Option<String>) -> ModelProviderInfo {
    let mut http_headers = HashMap::new();
    http_headers.insert("Content-Type".to_string(), "application/json".to_string());

    // Allow users to override the default Databricks endpoint by
    // exporting `DATABRICKS_BASE_URL`. This is similar to how OpenAI provider works.
    let effective_base_url = base_url.or_else(|| {
        std::env::var("DATABRICKS_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
    });

    ModelProviderInfo {
        name: "Databricks".into(),
        base_url: effective_base_url,
        env_key: Some("DATABRICKS_API_KEY".into()),
        env_key_instructions: Some(
            "Set the DATABRICKS_API_KEY environment variable with your Databricks API token. \
             You can get a token from your Databricks workspace settings."
                .into(),
        ),
        experimental_bearer_token: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: Some(http_headers),
        env_http_headers: None,
        request_max_retries: Some(DEFAULT_REQUEST_MAX_RETRIES),
        stream_max_retries: Some(DEFAULT_STREAM_MAX_RETRIES),
        stream_idle_timeout_ms: Some(DEFAULT_STREAM_IDLE_TIMEOUT_MS),
        requires_openai_auth: false,
        supports_websockets: false,
        uses_chat_completions_api: true, // Databricks uses Chat Completions API
    }
}

/// Creates an OpenRouter provider configuration.
///
/// OpenRouter is an API aggregator that provides access to many LLM models
/// (Claude, GPT-4o, Llama, Mistral, etc.) through a single API endpoint
/// that supports the OpenAI Responses API natively.
///
/// The API key should be set via the `OPENROUTER_API_KEY` environment variable.
/// Uses standard Bearer token authentication.
///
/// Example configuration in `~/.jarvis/config.toml`:
/// ```toml
/// [model_providers.openrouter]
/// name = "OpenRouter"
/// base_url = "https://openrouter.ai/api/v1"
/// env_key = "OPENROUTER_API_KEY"
/// wire_api = "responses"
/// ```
pub fn create_openrouter_provider() -> ModelProviderInfo {
    ModelProviderInfo {
        name: "OpenRouter".into(),
        base_url: Some("https://openrouter.ai/api/v1".into()),
        env_key: Some("OPENROUTER_API_KEY".into()),
        env_key_instructions: Some(
            "Set the OPENROUTER_API_KEY environment variable with your OpenRouter API key. \
             You can get one at https://openrouter.ai/keys"
                .into(),
        ),
        experimental_bearer_token: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: Some(DEFAULT_REQUEST_MAX_RETRIES),
        stream_max_retries: Some(DEFAULT_STREAM_MAX_RETRIES),
        stream_idle_timeout_ms: Some(DEFAULT_STREAM_IDLE_TIMEOUT_MS),
        requires_openai_auth: false,
        supports_websockets: false,
        uses_chat_completions_api: false, // OpenRouter supports Responses API natively
    }
}

/// Creates a Google AI Studio provider configuration.
///
/// Google AI Studio provides free-tier access to Gemini models through an
/// OpenAI-compatible Chat Completions API endpoint.
///
/// Free-tier models include: gemini-2.5-flash, gemini-2.5-flash-lite
///
/// The API key should be set via the `GOOGLE_API_KEY` environment variable.
/// Get a free key at: https://ai.google.dev
pub fn create_google_provider() -> ModelProviderInfo {
    let effective_base_url = std::env::var("GOOGLE_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta/openai".into());

    ModelProviderInfo {
        name: "Google AI Studio".into(),
        base_url: Some(effective_base_url),
        env_key: Some("GOOGLE_API_KEY".into()),
        env_key_instructions: Some(
            "Set the GOOGLE_API_KEY environment variable with your Google AI Studio API key. \
             You can get a free key at https://ai.google.dev"
                .into(),
        ),
        experimental_bearer_token: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: Some(DEFAULT_REQUEST_MAX_RETRIES),
        stream_max_retries: Some(DEFAULT_STREAM_MAX_RETRIES),
        stream_idle_timeout_ms: Some(DEFAULT_STREAM_IDLE_TIMEOUT_MS),
        requires_openai_auth: false,
        supports_websockets: false,
        uses_chat_completions_api: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_deserialize_ollama_model_provider_toml() {
        let azure_provider_toml = r#"
name = "Ollama"
base_url = "http://localhost:11434/v1"
        "#;
        let expected_provider = ModelProviderInfo {
            name: "Ollama".into(),
            base_url: Some("http://localhost:11434/v1".into()),
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            uses_chat_completions_api: false,
        };

        let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
        assert_eq!(expected_provider, provider);
    }

    #[test]
    fn test_deserialize_azure_model_provider_toml() {
        let azure_provider_toml = r#"
name = "Azure"
base_url = "https://xxxxx.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"
query_params = { api-version = "2025-04-01-preview" }
        "#;
        let expected_provider = ModelProviderInfo {
            name: "Azure".into(),
            base_url: Some("https://xxxxx.openai.azure.com/openai".into()),
            env_key: Some("AZURE_OPENAI_API_KEY".into()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            wire_api: WireApi::Responses,
            query_params: Some(maplit::hashmap! {
                "api-version".to_string() => "2025-04-01-preview".to_string(),
            }),
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            uses_chat_completions_api: false,
        };

        let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
        assert_eq!(expected_provider, provider);
    }

    #[test]
    fn test_deserialize_example_model_provider_toml() {
        let azure_provider_toml = r#"
name = "Example"
base_url = "https://example.com"
env_key = "API_KEY"
http_headers = { "X-Example-Header" = "example-value" }
env_http_headers = { "X-Example-Env-Header" = "EXAMPLE_ENV_VAR" }
        "#;
        let expected_provider = ModelProviderInfo {
            name: "Example".into(),
            base_url: Some("https://example.com".into()),
            env_key: Some("API_KEY".into()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: Some(maplit::hashmap! {
                "X-Example-Header".to_string() => "example-value".to_string(),
            }),
            env_http_headers: Some(maplit::hashmap! {
                "X-Example-Env-Header".to_string() => "EXAMPLE_ENV_VAR".to_string(),
            }),
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            uses_chat_completions_api: false,
        };

        let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
        assert_eq!(expected_provider, provider);
    }

    #[test]
    fn test_deserialize_chat_wire_api_shows_helpful_error() {
        let provider_toml = r#"
name = "OpenAI using Chat Completions"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"
        "#;

        let err = toml::from_str::<ModelProviderInfo>(provider_toml).unwrap_err();
        assert!(err.to_string().contains(CHAT_WIRE_API_REMOVED_ERROR));
    }

    #[test]
    fn test_create_databricks_provider() {
        let base_url = Some(
            "https://workspace.cloud.databricks.com/serving-endpoints/endpoint/invocations"
                .to_string(),
        );
        let provider = create_databricks_provider(base_url.clone());

        assert_eq!(provider.name, "Databricks");
        assert_eq!(provider.base_url, base_url);
        assert_eq!(provider.env_key, Some("DATABRICKS_API_KEY".to_string()));
        assert!(provider.env_key_instructions.is_some());
        assert_eq!(provider.wire_api, WireApi::Responses);
        assert!(provider.http_headers.is_some());
        assert!(
            provider
                .http_headers
                .as_ref()
                .unwrap()
                .contains_key("Content-Type")
        );
        assert_eq!(
            provider.http_headers.as_ref().unwrap().get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            provider.request_max_retries,
            Some(DEFAULT_REQUEST_MAX_RETRIES)
        );
        assert_eq!(
            provider.stream_max_retries,
            Some(DEFAULT_STREAM_MAX_RETRIES)
        );
        assert_eq!(
            provider.stream_idle_timeout_ms,
            Some(DEFAULT_STREAM_IDLE_TIMEOUT_MS)
        );
        assert!(!provider.requires_openai_auth);
        assert!(!provider.supports_websockets);
        assert!(provider.uses_chat_completions_api); // Databricks uses Chat Completions
    }

    #[test]
    fn test_deserialize_databricks_model_provider_toml() {
        let databricks_provider_toml = r#"
name = "Databricks"
base_url = "https://workspace.cloud.databricks.com/serving-endpoints/endpoint/invocations"
env_key = "DATABRICKS_API_KEY"
http_headers = { "Content-Type" = "application/json" }
        "#;
        let expected_provider = ModelProviderInfo {
            name: "Databricks".into(),
            base_url: Some(
                "https://workspace.cloud.databricks.com/serving-endpoints/endpoint/invocations"
                    .into(),
            ),
            env_key: Some("DATABRICKS_API_KEY".into()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: Some(maplit::hashmap! {
                "Content-Type".to_string() => "application/json".to_string(),
            }),
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            uses_chat_completions_api: false, // Default when not specified
        };

        let provider: ModelProviderInfo = toml::from_str(databricks_provider_toml).unwrap();
        assert_eq!(expected_provider, provider);
    }
}
