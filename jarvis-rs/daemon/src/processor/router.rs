//! LLM Router with automatic fallback support.
//!
//! This module implements a router that can automatically fallback to alternative
//! LLM providers when the primary provider fails (HTTP 429, 50x errors).
//! It includes circuit breaker functionality to avoid repeated attempts on
//! providers that are consistently failing.

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::DaemonPipeline;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::error;
use tracing::warn;

use super::LlmClient;
use super::LlmConfig;
use super::LlmProvider;
use super::LlmResponse;
use super::OpenAiCompatibleClient;

/// Classification of LLM errors to determine if fallback should be attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    /// HTTP 429 - Rate limit exceeded (recoverable, should trigger fallback)
    RateLimitExceeded,
    /// HTTP 50x - Server error (recoverable, should trigger fallback)
    ServerError,
    /// HTTP 400 - Bad request, typically context too large (not recoverable)
    ContextTooLarge,
    /// HTTP 401/403 - Authentication failed (not recoverable)
    AuthFailed,
    /// HTTP 403 - Quota/limit exceeded (recoverable, should trigger fallback to other providers)
    QuotaExceeded,
    /// Network errors, timeouts, connection issues (recoverable)
    NetworkError,
    /// Endpoint not supported (e.g., 404 for /responses when model only supports /chat/completions)
    /// This is recoverable and should trigger fallback or retry with different endpoint
    EndpointNotSupported,
    /// Other errors that may or may not be recoverable
    Other(String),
}

impl LlmError {
    /// Check if this error is recoverable and should trigger fallback.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded
                | Self::ServerError
                | Self::NetworkError
                | Self::EndpointNotSupported
                | Self::QuotaExceeded
        )
    }

    /// Classify an HTTP status code into an LlmError.
    pub fn from_status(status: StatusCode) -> Self {
        match status.as_u16() {
            429 => Self::RateLimitExceeded,
            400 | 413 => Self::ContextTooLarge,
            401 | 403 => Self::AuthFailed,
            500..=599 => Self::ServerError,
            _ => Self::Other(format!("HTTP {}", status.as_u16())),
        }
    }

    /// Classify an anyhow error into an LlmError.
    pub fn from_anyhow(err: &anyhow::Error) -> Self {
        let err_str = err.to_string().to_lowercase();
        if err_str.contains("timeout") || err_str.contains("connection") {
            Self::NetworkError
        } else if err_str.contains("429") {
            Self::RateLimitExceeded
        } else if err_str.contains("403")
            && (err_str.contains("limit exceeded")
                || err_str.contains("quota")
                || err_str.contains("key limit"))
        {
            // 403 with quota/limit exceeded is recoverable (should fallback to other providers)
            Self::QuotaExceeded
        } else if err_str.contains("401") || err_str.contains("403") {
            Self::AuthFailed
        } else if err_str.contains("400") || err_str.contains("413") {
            Self::ContextTooLarge
        } else if err_str.contains("50") {
            Self::ServerError
        } else if err_str.contains("404")
            && (err_str.contains("no endpoints found") || err_str.contains("endpoints found"))
        {
            // OpenRouter returns 404 with "No endpoints found" when model doesn't support /responses
            Self::EndpointNotSupported
        } else {
            Self::Other(err.to_string())
        }
    }
}

/// Configuration for a routing strategy (primary + fallbacks).
#[derive(Debug, Clone)]
pub struct StrategyConfig {
    /// Primary provider/model identifier (format: "provider/model")
    pub primary: String,
    /// Fallback provider/model identifiers in order of preference
    pub fallbacks: Vec<String>,
}

/// Circuit breaker state for a provider/model.
#[derive(Debug, Clone)]
struct CircuitBreakerState {
    /// Number of consecutive failures
    failures: u32,
    /// Timestamp of last failure
    last_failure: Option<Instant>,
    /// Whether the circuit is currently open (blocking requests)
    is_open: bool,
}

/// Detailed metrics for a provider/model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    /// Total number of requests attempted
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Last successful request timestamp (Unix seconds)
    pub last_success_at: Option<i64>,
    /// Last failure timestamp (Unix seconds)
    pub last_failure_at: Option<i64>,
    /// Historical metrics by hour (for temporal analysis)
    #[serde(default)]
    pub hourly_metrics: HashMap<String, HourlyMetrics>,
    /// Historical metrics by day (for temporal analysis)
    #[serde(default)]
    pub daily_metrics: HashMap<String, DailyMetrics>,
    /// Sum of all latencies (for calculating average)
    #[serde(skip)]
    latency_sum_ms: f64,
    /// Number of latency samples
    #[serde(skip)]
    latency_samples: u64,
}

/// Metrics aggregated by hour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyMetrics {
    /// Hour identifier (format: "YYYY-MM-DD-HH")
    pub hour: String,
    /// Requests in this hour
    pub requests: u64,
    /// Successful requests
    pub successful: u64,
    /// Failed requests
    pub failed: u64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Total cost (USD)
    pub cost_usd: f64,
}

/// Metrics aggregated by day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyMetrics {
    /// Day identifier (format: "YYYY-MM-DD")
    pub day: String,
    /// Requests in this day
    pub requests: u64,
    /// Successful requests
    pub successful: u64,
    /// Failed requests
    pub failed: u64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Total cost (USD)
    pub cost_usd: f64,
}

impl ProviderMetrics {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0.0,
            total_cost_usd: 0.0,
            last_success_at: None,
            last_failure_at: None,
            hourly_metrics: HashMap::new(),
            daily_metrics: HashMap::new(),
            latency_sum_ms: 0.0,
            latency_samples: 0,
        }
    }

    /// Record a successful request.
    fn record_success(&mut self, latency_ms: f64, cost_usd: Option<f64>) {
        let now = chrono::Utc::now();
        let timestamp = now.timestamp();

        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_success_at = Some(timestamp);

        // Update average latency
        self.latency_sum_ms += latency_ms;
        self.latency_samples += 1;
        self.avg_latency_ms = self.latency_sum_ms / self.latency_samples as f64;

        let cost = cost_usd.unwrap_or(0.0);
        if cost > 0.0 {
            self.total_cost_usd += cost;
        }

        // Update temporal metrics
        self.update_temporal_metrics(true, latency_ms, cost, &now);
    }

    /// Record a failed request.
    fn record_failure(&mut self, latency_ms: f64) {
        let now = chrono::Utc::now();
        let timestamp = now.timestamp();

        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_failure_at = Some(timestamp);

        // Update average latency (even failures have latency)
        self.latency_sum_ms += latency_ms;
        self.latency_samples += 1;
        self.avg_latency_ms = self.latency_sum_ms / self.latency_samples as f64;

        // Update temporal metrics
        self.update_temporal_metrics(false, latency_ms, 0.0, &now);
    }

    /// Update hourly and daily metrics.
    fn update_temporal_metrics(
        &mut self,
        success: bool,
        latency_ms: f64,
        cost_usd: f64,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) {
        // Hourly metrics
        let hour_key = timestamp.format("%Y-%m-%d-%H").to_string();
        let hourly = self
            .hourly_metrics
            .entry(hour_key.clone())
            .or_insert_with(|| HourlyMetrics {
                hour: hour_key,
                requests: 0,
                successful: 0,
                failed: 0,
                avg_latency_ms: 0.0,
                cost_usd: 0.0,
            });
        hourly.requests += 1;
        if success {
            hourly.successful += 1;
        } else {
            hourly.failed += 1;
        }
        // Update average latency (simple moving average)
        hourly.avg_latency_ms = (hourly.avg_latency_ms * (hourly.requests - 1) as f64 + latency_ms)
            / hourly.requests as f64;
        hourly.cost_usd += cost_usd;

        // Daily metrics
        let day_key = timestamp.format("%Y-%m-%d").to_string();
        let daily = self
            .daily_metrics
            .entry(day_key.clone())
            .or_insert_with(|| DailyMetrics {
                day: day_key,
                requests: 0,
                successful: 0,
                failed: 0,
                avg_latency_ms: 0.0,
                cost_usd: 0.0,
            });
        daily.requests += 1;
        if success {
            daily.successful += 1;
        } else {
            daily.failed += 1;
        }
        // Update average latency
        daily.avg_latency_ms = (daily.avg_latency_ms * (daily.requests - 1) as f64 + latency_ms)
            / daily.requests as f64;
        daily.cost_usd += cost_usd;
    }

    /// Get hourly metrics for a specific hour or recent hours.
    pub fn get_hourly_metrics(&self, hour: Option<&str>) -> Vec<&HourlyMetrics> {
        if let Some(h) = hour {
            self.hourly_metrics.get(h).into_iter().collect()
        } else {
            // Return last 24 hours
            let mut hours: Vec<_> = self.hourly_metrics.values().collect();
            hours.sort_by(|a, b| b.hour.cmp(&a.hour));
            hours.into_iter().take(24).collect()
        }
    }

    /// Get daily metrics for a specific day or recent days.
    pub fn get_daily_metrics(&self, day: Option<&str>) -> Vec<&DailyMetrics> {
        if let Some(d) = day {
            self.daily_metrics.get(d).into_iter().collect()
        } else {
            // Return last 30 days
            let mut days: Vec<_> = self.daily_metrics.values().collect();
            days.sort_by(|a, b| b.day.cmp(&a.day));
            days.into_iter().take(30).collect()
        }
    }

    /// Get success rate as a percentage (0.0 to 100.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.successful_requests as f64 / self.total_requests as f64) * 100.0
    }

    /// Get failure rate as a percentage (0.0 to 100.0).
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.failed_requests as f64 / self.total_requests as f64) * 100.0
    }
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            failures: 0,
            last_failure: None,
            is_open: false,
        }
    }

    /// Check if the circuit breaker allows a request.
    fn can_attempt(&self) -> bool {
        if !self.is_open {
            return true;
        }

        // Check if cooldown period has passed (15 minutes)
        if let Some(last_failure) = self.last_failure {
            if last_failure.elapsed() >= Duration::from_secs(15 * 60) {
                return true;
            }
        }

        false
    }

    /// Record a failure.
    fn record_failure(&mut self) {
        self.failures += 1;
        self.last_failure = Some(Instant::now());

        // Open circuit after 3 consecutive failures
        if self.failures >= 3 {
            self.is_open = true;
        }
    }

    /// Record a success, resetting the failure count.
    fn record_success(&mut self) {
        self.failures = 0;
        self.is_open = false;
        self.last_failure = None;
    }
}

/// LLM Router that implements automatic fallback between providers.
pub struct LlmRouter {
    /// Map of strategy names to their configurations
    strategies: HashMap<String, StrategyConfig>,
    /// Circuit breakers per provider/model identifier
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreakerState>>>,
    /// Detailed metrics per provider/model identifier
    metrics: Arc<RwLock<HashMap<String, ProviderMetrics>>>,
    /// Fallback logging enabled
    log_fallbacks: bool,
    /// Metrics persistence enabled
    persist_metrics: bool,
    /// Alert thresholds
    alert_thresholds: AlertThresholds,
}

/// Alert thresholds for monitoring provider performance.
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// Minimum success rate (0.0 to 100.0) before alerting.
    pub min_success_rate: f64,
    /// Maximum average latency (ms) before alerting.
    pub max_avg_latency_ms: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            min_success_rate: 80.0,     // Alert if success rate drops below 80%
            max_avg_latency_ms: 5000.0, // Alert if avg latency exceeds 5 seconds
        }
    }
}

impl LlmRouter {
    /// Create a new router with strategies loaded from config.toml (if available) or default hardcoded strategies.
    pub fn new() -> Self {
        let strategies = Self::load_strategies_from_config().unwrap_or_else(|| {
            // Fallback to hardcoded strategies if config.toml is not available or doesn't have strategies
            Self::default_strategies()
        });

        let router = Self {
            strategies,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            log_fallbacks: true,
            persist_metrics: true,
            alert_thresholds: AlertThresholds::default(),
        };

        // Load persisted metrics in background
        let router_clone = router.clone();
        tokio::spawn(async move {
            if let Err(e) = router_clone.load_metrics().await {
                warn!("Failed to load persisted metrics: {}", e);
            }
        });

        router
    }

    /// Load strategies from config.toml file.
    /// Returns None if config.toml doesn't exist or doesn't have llm.strategies section.
    fn load_strategies_from_config() -> Option<HashMap<String, StrategyConfig>> {
        use std::fs;
        use toml;

        // Try to find and load config.toml
        let jarvis_home = dirs::home_dir()?.join(".jarvis");
        let config_file = jarvis_home.join("config.toml");

        if !config_file.exists() {
            return None;
        }

        let contents = match fs::read_to_string(&config_file) {
            Ok(c) => c,
            Err(_) => return None,
        };

        let config: toml::Value = match toml::from_str(&contents) {
            Ok(c) => c,
            Err(_) => return None,
        };

        // Extract llm.strategies section
        let llm_config = config.get("llm")?.get("strategies")?;

        let mut strategies = HashMap::new();

        if let Some(strategies_table) = llm_config.as_table() {
            for (name, strategy_value) in strategies_table {
                if let Some(strategy_table) = strategy_value.as_table() {
                    let primary = strategy_table.get("primary")?.as_str()?.to_string();
                    let fallbacks = strategy_table
                        .get("fallbacks")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    strategies.insert(name.clone(), StrategyConfig { primary, fallbacks });
                }
            }
        }

        if strategies.is_empty() {
            None
        } else {
            Some(strategies)
        }
    }

    /// Default hardcoded strategies (used as fallback).
    fn default_strategies() -> HashMap<String, StrategyConfig> {
        let mut strategies = HashMap::new();

        // heavy_context: For tasks requiring long context (code reading, video transcription)
        strategies.insert(
            "heavy_context".to_string(),
            StrategyConfig {
                primary: "google/gemini-2.0-flash".to_string(),
                fallbacks: vec![
                    "openrouter/google/gemini-2.0-flash:free".to_string(),
                    "github/gpt-4o-mini".to_string(),
                ],
            },
        );

        // reasoning: For tasks requiring logical reasoning (code review, planning)
        strategies.insert(
            "reasoning".to_string(),
            StrategyConfig {
                primary: "openrouter/free".to_string(),
                fallbacks: vec![
                    "groq/llama-3.3-70b-versatile".to_string(),
                    "google/gemini-2.0-flash".to_string(),
                ],
            },
        );

        // fast_routing: For quick routing decisions (deciding which agent to call)
        strategies.insert(
            "fast_routing".to_string(),
            StrategyConfig {
                primary: "groq/llama-3.3-70b-versatile".to_string(),
                fallbacks: vec!["github/gpt-4o-mini".to_string()],
            },
        );

        strategies
    }

    /// Create a router from a pipeline configuration.
    ///
    /// If the pipeline config specifies a strategy, uses that strategy with fallback.
    /// If auto_strategy is enabled, automatically selects strategy based on pipeline context.
    /// Otherwise, creates a single-provider client (backward compatible).
    pub async fn from_pipeline_config(pipeline: &DaemonPipeline) -> Result<Arc<dyn LlmClient>> {
        let config_value: serde_json::Value = serde_json::from_str(&pipeline.config_json)?;
        let llm_config: LlmConfig = config_value
            .get("llm")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or_default();

        let router = Self::new();

        // Check if strategy is explicitly specified
        if let Some(strategy_name) = config_value
            .get("llm")
            .and_then(|v| v.get("strategy"))
            .and_then(|v| v.as_str())
        {
            let use_auto_tune = config_value
                .get("llm")
                .and_then(|v| v.get("auto_tune"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let client = router
                .create_strategy_client(strategy_name, &llm_config, use_auto_tune)
                .await?;
            Ok(Arc::new(client))
        }
        // Check if auto_strategy is enabled
        else if config_value
            .get("llm")
            .and_then(|v| v.get("auto_strategy"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            // Auto-select strategy based on pipeline context
            let prompt_length = config_value
                .get("llm")
                .and_then(|v| v.get("prompt_length"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let requires_reasoning = config_value
                .get("llm")
                .and_then(|v| v.get("requires_reasoning"))
                .and_then(|v| v.as_bool());

            if let Some(strategy_name) = router.select_strategy_for_context(
                &pipeline.strategy,
                prompt_length,
                requires_reasoning,
            ) {
                let use_auto_tune = config_value
                    .get("llm")
                    .and_then(|v| v.get("auto_tune"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true); // Default to true for auto_strategy mode

                let client = router
                    .create_strategy_client(&strategy_name, &llm_config, use_auto_tune)
                    .await?;
                Ok(Arc::new(client))
            } else {
                // Fallback to single provider if no strategy matches
                let client = OpenAiCompatibleClient::from_config(llm_config)?;
                Ok(Arc::new(client))
            }
        } else {
            // Backward compatible: use single provider
            let client = OpenAiCompatibleClient::from_config(llm_config)?;
            Ok(Arc::new(client))
        }
    }

    /// Create a client for a specific strategy.
    /// Optionally uses auto-tuning if enabled.
    async fn create_strategy_client(
        &self,
        strategy_name: &str,
        base_config: &LlmConfig,
        use_auto_tune: bool,
    ) -> Result<StrategyClient> {
        let strategy = if use_auto_tune {
            // Try to get auto-tuned strategy
            if let Ok(Some(tuned_strategy)) = self.get_auto_tuned_strategy(strategy_name).await {
                tuned_strategy
            } else {
                // Fallback to original strategy if auto-tuning fails
                self.strategies
                    .get(strategy_name)
                    .ok_or_else(|| anyhow::anyhow!("Unknown strategy: {}", strategy_name))?
                    .clone()
            }
        } else {
            self.strategies
                .get(strategy_name)
                .ok_or_else(|| anyhow::anyhow!("Unknown strategy: {}", strategy_name))?
                .clone()
        };

        Ok(StrategyClient {
            strategy_name: strategy_name.to_string(),
            strategy,
            base_config: base_config.clone(),
            router: self.clone(),
        })
    }

    /// Parse a provider/model identifier into LlmConfig.
    ///
    /// Format: "provider/model" or "provider/model:variant"
    fn parse_provider_model(&self, identifier: &str) -> Result<LlmConfig> {
        let parts: Vec<&str> = identifier.split('/').collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid provider/model format: {}", identifier);
        }

        let provider_str = parts[0];
        let model_with_variant = parts[1..].join("/");

        let provider = match provider_str {
            "google" => LlmProvider::Google,
            "openrouter" => LlmProvider::Openrouter,
            "groq" => LlmProvider::Custom, // Groq uses custom endpoint
            "github" => LlmProvider::Openai, // GitHub Models uses OpenAI-compatible API
            _ => anyhow::bail!("Unknown provider: {}", provider_str),
        };

        // Handle Groq special case
        let (base_url, model, api_key_env) = if provider_str == "groq" {
            (
                Some("https://api.groq.com/openai/v1".to_string()),
                model_with_variant,
                Some("GROQ_API_KEY".to_string()),
            )
        } else if provider_str == "github" {
            (
                Some("https://models.inference.ai/v1".to_string()),
                model_with_variant,
                Some("GITHUB_MODELS_API_KEY".to_string()),
            )
        } else {
            (None, model_with_variant, None)
        };

        // Try to resolve API key from environment if needed
        let api_key = if let Some(env_var) = &api_key_env {
            std::env::var(env_var).ok()
        } else {
            None
        };

        Ok(LlmConfig {
            provider,
            model: Some(model),
            base_url,
            api_key,
            max_tokens: 4096,
            temperature: 0.7,
            extra_headers: HashMap::new(),
        })
    }

    /// Check if circuit breaker allows attempting a provider.
    async fn can_attempt_provider(&self, identifier: &str) -> bool {
        let breakers = self.circuit_breakers.read().await;
        if let Some(state) = breakers.get(identifier) {
            state.can_attempt()
        } else {
            true
        }
    }

    /// Record a failure for a provider.
    async fn record_provider_failure(&self, identifier: &str) {
        let mut breakers = self.circuit_breakers.write().await;
        let state = breakers
            .entry(identifier.to_string())
            .or_insert_with(CircuitBreakerState::new);
        state.record_failure();
    }

    /// Record a success for a provider.
    async fn record_provider_success(&self, identifier: &str) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(state) = breakers.get_mut(identifier) {
            state.record_success();
        }
    }

    /// Record metrics for a successful request.
    async fn record_metrics_success(
        &self,
        identifier: &str,
        latency_ms: f64,
        cost_usd: Option<f64>,
    ) {
        let mut metrics = self.metrics.write().await;
        let provider_metrics = metrics
            .entry(identifier.to_string())
            .or_insert_with(ProviderMetrics::new);
        provider_metrics.record_success(latency_ms, cost_usd);

        // Check for alerts
        self.check_alerts(identifier, provider_metrics).await;

        // Persist metrics if enabled
        if self.persist_metrics {
            if let Err(e) = self.save_metrics().await {
                warn!("Failed to persist metrics: {}", e);
            }
        }
    }

    /// Record metrics for a failed request.
    async fn record_metrics_failure(&self, identifier: &str, latency_ms: f64) {
        let mut metrics = self.metrics.write().await;
        let provider_metrics = metrics
            .entry(identifier.to_string())
            .or_insert_with(ProviderMetrics::new);
        provider_metrics.record_failure(latency_ms);

        // Check for alerts
        self.check_alerts(identifier, provider_metrics).await;

        // Persist metrics if enabled
        if self.persist_metrics {
            if let Err(e) = self.save_metrics().await {
                warn!("Failed to persist metrics: {}", e);
            }
        }
    }

    /// Check metrics against alert thresholds and log warnings if needed.
    async fn check_alerts(&self, identifier: &str, metrics: &ProviderMetrics) {
        // Only alert if we have enough data (at least 10 requests)
        if metrics.total_requests < 10 {
            return;
        }

        let mut alerts = Vec::new();

        // Check success rate
        if metrics.success_rate() < self.alert_thresholds.min_success_rate {
            alerts.push(format!(
                "Low success rate: {:.2}% (threshold: {:.2}%)",
                metrics.success_rate(),
                self.alert_thresholds.min_success_rate
            ));
        }

        // Check latency
        if metrics.avg_latency_ms > self.alert_thresholds.max_avg_latency_ms {
            alerts.push(format!(
                "High latency: {:.2}ms (threshold: {:.2}ms)",
                metrics.avg_latency_ms, self.alert_thresholds.max_avg_latency_ms
            ));
        }

        // Log alerts
        if !alerts.is_empty() {
            error!(
                "⚠️  ALERT for provider {}: {}",
                identifier,
                alerts.join(", ")
            );

            // Also write to alert log file
            if let Err(e) = self.write_alert_log(identifier, &alerts).await {
                warn!("Failed to write alert log: {}", e);
            }
        }
    }

    /// Write alert to log file.
    async fn write_alert_log(&self, identifier: &str, alerts: &[String]) -> Result<()> {
        let jarvis_home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".jarvis");

        let logs_dir = jarvis_home.join("logs");
        tokio::fs::create_dir_all(&logs_dir).await?;

        let alert_file = logs_dir.join("llm_alerts.log");
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&alert_file)
            .await?;

        let timestamp = chrono::Utc::now().to_rfc3339();
        let alert_line = format!(
            "{} [ALERT] provider={} {}\n",
            timestamp,
            identifier,
            alerts.join("; ")
        );

        use tokio::io::AsyncWriteExt;
        file.write_all(alert_line.as_bytes()).await?;
        Ok(())
    }

    /// Get metrics for a specific provider.
    pub async fn get_provider_metrics(&self, identifier: &str) -> Option<ProviderMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(identifier).cloned()
    }

    /// Get all provider metrics.
    pub async fn get_all_metrics(&self) -> HashMap<String, ProviderMetrics> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Load metrics from disk.
    pub async fn load_metrics(&self) -> Result<()> {
        let jarvis_home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".jarvis");
        let metrics_file = jarvis_home.join("llm_metrics.json");

        if !metrics_file.exists() {
            return Ok(()); // No metrics file yet, that's fine
        }

        let contents = tokio::fs::read_to_string(&metrics_file).await?;
        let loaded: HashMap<String, ProviderMetrics> = serde_json::from_str(&contents)?;

        let mut metrics = self.metrics.write().await;
        // Merge with existing metrics (loaded metrics take precedence for persisted data)
        for (key, value) in loaded {
            metrics.insert(key, value);
        }

        Ok(())
    }

    /// Save metrics to disk.
    async fn save_metrics(&self) -> Result<()> {
        let jarvis_home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".jarvis");
        tokio::fs::create_dir_all(&jarvis_home).await?;

        let metrics_file = jarvis_home.join("llm_metrics.json");
        let metrics = self.metrics.read().await;
        let serialized = serde_json::to_string_pretty(&*metrics)?;
        tokio::fs::write(&metrics_file, serialized).await?;

        Ok(())
    }

    /// Auto-tune strategy based on performance metrics.
    /// Returns a reordered strategy with best performers first.
    /// This doesn't modify the original strategy, but returns an optimized version.
    pub async fn get_auto_tuned_strategy(
        &self,
        strategy_name: &str,
    ) -> Result<Option<StrategyConfig>> {
        let strategy = self
            .strategies
            .get(strategy_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown strategy: {}", strategy_name))?;

        let metrics = self.metrics.read().await;

        // Score each provider based on performance
        let mut provider_scores: Vec<(String, f64)> = vec![];

        // Score primary
        if let Some(primary_metrics) = metrics.get(&strategy.primary) {
            let score = Self::calculate_provider_score(primary_metrics);
            provider_scores.push((strategy.primary.clone(), score));
        } else {
            // No metrics yet, use default score
            provider_scores.push((strategy.primary.clone(), 50.0));
        }

        // Score fallbacks
        for fallback in &strategy.fallbacks {
            if let Some(fallback_metrics) = metrics.get(fallback) {
                let score = Self::calculate_provider_score(fallback_metrics);
                provider_scores.push((fallback.clone(), score));
            } else {
                provider_scores.push((fallback.clone(), 50.0));
            }
        }

        // Sort by score (higher is better)
        provider_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return reordered strategy (best performer becomes primary, rest become fallbacks)
        if !provider_scores.is_empty() {
            Ok(Some(StrategyConfig {
                primary: provider_scores[0].0.clone(),
                fallbacks: provider_scores[1..]
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Calculate a performance score for a provider (0.0 to 100.0).
    /// Higher score = better performance.
    fn calculate_provider_score(metrics: &ProviderMetrics) -> f64 {
        if metrics.total_requests == 0 {
            return 50.0; // Default score for providers with no history
        }

        let success_rate = metrics.success_rate();
        let latency_score = if metrics.avg_latency_ms > 0.0 {
            // Lower latency = higher score (max 1000ms = 0 score, 0ms = 100 score)
            (1000.0 - metrics.avg_latency_ms.min(1000.0)) / 10.0
        } else {
            50.0
        };

        // Weighted combination: 70% success rate, 30% latency
        (success_rate * 0.7) + (latency_score * 0.3)
    }

    /// Select the best strategy based on pipeline context.
    /// Analyzes the pipeline type and prompt characteristics to choose an appropriate strategy.
    pub fn select_strategy_for_context(
        &self,
        pipeline_strategy: &str,
        prompt_length: Option<usize>,
        requires_reasoning: Option<bool>,
    ) -> Option<String> {
        // Analyze context to determine best strategy
        let is_long_context = prompt_length.map(|len| len > 10000).unwrap_or(false);
        let needs_reasoning = requires_reasoning.unwrap_or(false);

        // Map pipeline types to strategies
        match pipeline_strategy {
            "seo_blog" | "youtube_shorts" => {
                // Content generation typically needs long context
                Some("heavy_context".to_string())
            }
            "strategy_analyzer" | "ab_tester" => {
                // Analysis tasks need reasoning
                Some("reasoning".to_string())
            }
            "metrics_collector" | "prompt_optimizer" => {
                // Quick tasks
                Some("fast_routing".to_string())
            }
            _ => {
                // Dynamic selection based on prompt characteristics
                if is_long_context {
                    Some("heavy_context".to_string())
                } else if needs_reasoning {
                    Some("reasoning".to_string())
                } else {
                    Some("fast_routing".to_string())
                }
            }
        }
    }

    /// Log a fallback event.
    async fn log_fallback(&self, strategy: &str, failed: &str, using: &str, error: &LlmError) {
        if self.log_fallbacks {
            warn!(
                "LLM fallback triggered: strategy={}, failed={}, using={}, error={:?}",
                strategy, failed, using, error
            );

            // Also log to file
            if let Err(e) = self
                .write_fallback_log(strategy, failed, using, error)
                .await
            {
                warn!("Failed to write fallback log: {}", e);
            }
        }
    }

    /// Write fallback event to log file.
    async fn write_fallback_log(
        &self,
        strategy: &str,
        failed: &str,
        using: &str,
        error: &LlmError,
    ) -> Result<()> {
        // Get jarvis home directory
        let jarvis_home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".jarvis");

        // Create logs directory if it doesn't exist
        let logs_dir = jarvis_home.join("logs");
        tokio::fs::create_dir_all(&logs_dir).await?;

        // Append to log file
        let log_file = logs_dir.join("llm_fallback.log");
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .await?;

        let timestamp = chrono::Utc::now().to_rfc3339();
        let log_line = format!(
            "{} strategy={} failed={} using={} error={:?}\n",
            timestamp, strategy, failed, using, error
        );

        use tokio::io::AsyncWriteExt;
        file.write_all(log_line.as_bytes()).await?;
        Ok(())
    }
}

impl Clone for LlmRouter {
    fn clone(&self) -> Self {
        Self {
            strategies: self.strategies.clone(),
            circuit_breakers: self.circuit_breakers.clone(),
            metrics: self.metrics.clone(),
            log_fallbacks: self.log_fallbacks,
            persist_metrics: self.persist_metrics,
            alert_thresholds: self.alert_thresholds.clone(),
        }
    }
}

/// Client wrapper that implements LlmClient for a specific strategy.
struct StrategyClient {
    strategy_name: String,
    strategy: StrategyConfig,
    base_config: LlmConfig,
    router: LlmRouter,
}

#[async_trait]
impl LlmClient for StrategyClient {
    async fn generate(&self, prompt: &str, system: Option<&str>) -> Result<LlmResponse> {
        // Build list of providers to try (primary + fallbacks)
        let mut providers = vec![self.strategy.primary.clone()];
        providers.extend(self.strategy.fallbacks.iter().cloned());

        let mut last_error: Option<anyhow::Error> = None;

        for (idx, provider_id) in providers.iter().enumerate() {
            // Check circuit breaker
            if !self.router.can_attempt_provider(provider_id).await {
                warn!(
                    "Skipping {} due to circuit breaker (strategy={})",
                    provider_id, self.strategy_name
                );
                continue;
            }

            // Parse provider/model config
            let mut config = match self.router.parse_provider_model(provider_id) {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to parse provider {}: {}", provider_id, e);
                    continue;
                }
            };

            // Merge with base config (base config takes precedence for some fields)
            config.max_tokens = self.base_config.max_tokens;
            config.temperature = self.base_config.temperature;
            if self.base_config.api_key.is_some() {
                config.api_key = self.base_config.api_key.clone();
            }

            // Create client and attempt request
            match OpenAiCompatibleClient::from_config(config.clone()) {
                Ok(client) => {
                    let start_time = Instant::now();
                    match client.generate(prompt, system).await {
                        Ok(response) => {
                            let elapsed = start_time.elapsed();
                            let latency_ms = elapsed.as_secs_f64() * 1000.0;

                            // Record success in circuit breaker
                            self.router.record_provider_success(provider_id).await;

                            // Record metrics
                            self.router
                                .record_metrics_success(provider_id, latency_ms, response.cost_usd)
                                .await;

                            // Log fallback if this wasn't the primary
                            if idx > 0 {
                                self.router
                                    .log_fallback(
                                        &self.strategy_name,
                                        &providers[0],
                                        provider_id,
                                        &LlmError::Other("Success after fallback".to_string()),
                                    )
                                    .await;
                            }

                            return Ok(response);
                        }
                        Err(e) => {
                            let elapsed = start_time.elapsed();
                            let latency_ms = elapsed.as_secs_f64() * 1000.0;

                            // Record metrics for failure
                            self.router
                                .record_metrics_failure(provider_id, latency_ms)
                                .await;

                            // Extract HTTP status from error message
                            // Format: "LLM API error {status} ({provider}, {model}):\n{text}"
                            let error_msg = e.to_string();
                            let error_msg_lower = error_msg.to_lowercase();
                            let llm_error = if error_msg.contains("429") {
                                LlmError::RateLimitExceeded
                            } else if error_msg.contains("403")
                                && (error_msg_lower.contains("limit exceeded")
                                    || error_msg_lower.contains("quota")
                                    || error_msg_lower.contains("key limit"))
                            {
                                // 403 with quota/limit exceeded is recoverable (should fallback to other providers)
                                LlmError::QuotaExceeded
                            } else if error_msg.contains("401") || error_msg.contains("403") {
                                LlmError::AuthFailed
                            } else if error_msg.contains("400") || error_msg.contains("413") {
                                LlmError::ContextTooLarge
                            } else if error_msg.contains("500")
                                || error_msg.contains("502")
                                || error_msg.contains("503")
                                || error_msg.contains("504")
                            {
                                LlmError::ServerError
                            } else if error_msg.contains("timeout")
                                || error_msg.contains("connection")
                                || error_msg.contains("network")
                            {
                                LlmError::NetworkError
                            } else if error_msg.contains("404")
                                && (error_msg_lower.contains("no endpoints found")
                                    || error_msg_lower.contains("endpoints found"))
                            {
                                // OpenRouter returns 404 with "No endpoints found" when model doesn't support /responses
                                LlmError::EndpointNotSupported
                            } else {
                                // Try to parse status code from message
                                let status_code = error_msg.split_whitespace().find_map(|s| {
                                    // Look for patterns like "HTTP 429" or just "429"
                                    if s.starts_with("HTTP") {
                                        s.split_whitespace().nth(1)?.parse::<u16>().ok()
                                    } else {
                                        s.parse::<u16>().ok()
                                    }
                                });

                                if let Some(code) = status_code {
                                    if let Ok(status) = StatusCode::from_u16(code) {
                                        // For 404, check if it's an endpoint not supported error
                                        if code == 404
                                            && (error_msg_lower.contains("no endpoints found")
                                                || error_msg_lower.contains("endpoints found"))
                                        {
                                            LlmError::EndpointNotSupported
                                        } else {
                                            LlmError::from_status(status)
                                        }
                                    } else {
                                        LlmError::from_anyhow(&e)
                                    }
                                } else {
                                    LlmError::from_anyhow(&e)
                                }
                            };

                            // Check if error is recoverable
                            if llm_error.is_recoverable() {
                                warn!(
                                    "Provider {} failed (recoverable): {} (took {:?})",
                                    provider_id, e, elapsed
                                );

                                // Record failure for circuit breaker
                                self.router.record_provider_failure(provider_id).await;

                                // Log fallback if this was primary
                                if idx == 0 && providers.len() > 1 {
                                    self.router
                                        .log_fallback(
                                            &self.strategy_name,
                                            provider_id,
                                            &providers[1],
                                            &llm_error,
                                        )
                                        .await;
                                }

                                last_error = Some(e);
                                continue; // Try next provider
                            } else {
                                // Non-recoverable error, fail immediately
                                error!(
                                    "Provider {} failed with non-recoverable error: {}",
                                    provider_id, e
                                );
                                return Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to create client for {}: {}", provider_id, e);
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // All providers failed
        error!(
            "All providers failed for strategy {}: {:?}",
            self.strategy_name,
            last_error.as_ref().map(|e| e.to_string())
        );

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("All providers in strategy '{}' failed", self.strategy_name)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_llm_error_classification() {
        assert!(LlmError::RateLimitExceeded.is_recoverable());
        assert!(LlmError::ServerError.is_recoverable());
        assert!(LlmError::NetworkError.is_recoverable());
        assert!(LlmError::QuotaExceeded.is_recoverable());
        assert!(LlmError::EndpointNotSupported.is_recoverable());
        assert!(!LlmError::ContextTooLarge.is_recoverable());
        assert!(!LlmError::AuthFailed.is_recoverable());
        assert!(!LlmError::Other("test".to_string()).is_recoverable());
    }

    #[test]
    fn test_llm_error_from_status() {
        assert_eq!(
            LlmError::from_status(StatusCode::TOO_MANY_REQUESTS),
            LlmError::RateLimitExceeded
        );
        assert_eq!(
            LlmError::from_status(StatusCode::INTERNAL_SERVER_ERROR),
            LlmError::ServerError
        );
        assert_eq!(
            LlmError::from_status(StatusCode::BAD_REQUEST),
            LlmError::ContextTooLarge
        );
        assert_eq!(
            LlmError::from_status(StatusCode::PAYLOAD_TOO_LARGE),
            LlmError::ContextTooLarge
        );
        assert_eq!(
            LlmError::from_status(StatusCode::UNAUTHORIZED),
            LlmError::AuthFailed
        );
        assert_eq!(
            LlmError::from_status(StatusCode::FORBIDDEN),
            LlmError::AuthFailed
        );
    }

    #[test]
    fn test_llm_error_from_anyhow() {
        // Test timeout error
        let timeout_err = anyhow!("request timeout after 30s");
        assert_eq!(LlmError::from_anyhow(&timeout_err), LlmError::NetworkError);

        // Test connection error
        let conn_err = anyhow!("connection refused");
        assert_eq!(LlmError::from_anyhow(&conn_err), LlmError::NetworkError);

        // Test HTTP 429 error
        let rate_limit_err =
            anyhow!("LLM API error 429 (google, gemini-2.0-flash):\nRate limit exceeded");
        assert_eq!(
            LlmError::from_anyhow(&rate_limit_err),
            LlmError::RateLimitExceeded
        );

        // Test HTTP 401 error
        let auth_err = anyhow!("LLM API error 401 (openai, gpt-4o-mini):\nUnauthorized");
        assert_eq!(LlmError::from_anyhow(&auth_err), LlmError::AuthFailed);

        // Test HTTP 500 error
        let server_err =
            anyhow!("LLM API error 500 (openrouter, mistral-nemo):\nInternal server error");
        assert_eq!(LlmError::from_anyhow(&server_err), LlmError::ServerError);

        // Test HTTP 403 with quota/limit exceeded (should be QuotaExceeded, recoverable)
        let quota_err = anyhow!(
            "LLM API error 403 (openrouter, openrouter/free):\nKey limit exceeded (monthly limit)"
        );
        assert_eq!(LlmError::from_anyhow(&quota_err), LlmError::QuotaExceeded);

        // Test HTTP 403 without quota/limit message (should be AuthFailed, not recoverable)
        let auth_403_err = anyhow!("LLM API error 403 (openrouter, openrouter/free):\nForbidden");
        assert_eq!(LlmError::from_anyhow(&auth_403_err), LlmError::AuthFailed);
    }

    #[test]
    fn test_circuit_breaker_state() {
        let mut state = CircuitBreakerState::new();
        assert!(state.can_attempt());

        // Record 2 failures - still should allow
        state.record_failure();
        state.record_failure();
        assert!(state.can_attempt());
        assert_eq!(state.failures, 2);

        // Record 3rd failure - should open circuit
        state.record_failure();
        assert!(!state.can_attempt());
        assert!(state.is_open);
        assert_eq!(state.failures, 3);

        // Record success - should close circuit
        state.record_success();
        assert!(state.can_attempt());
        assert!(!state.is_open);
        assert_eq!(state.failures, 0);
        assert!(state.last_failure.is_none());
    }

    #[test]
    fn test_circuit_breaker_cooldown() {
        let mut state = CircuitBreakerState::new();

        // Open circuit
        state.record_failure();
        state.record_failure();
        state.record_failure();
        assert!(!state.can_attempt());

        // Simulate time passing (we can't actually wait, but we can test the logic)
        // The cooldown check happens in can_attempt(), which checks if 15 minutes have passed
        // In a real scenario, we'd need to use a mock time, but for now we test the structure
        assert!(state.last_failure.is_some());
    }

    #[tokio::test]
    async fn test_router_default_strategies() {
        let router = LlmRouter::new();
        assert!(router.strategies.contains_key("heavy_context"));
        assert!(router.strategies.contains_key("reasoning"));
        assert!(router.strategies.contains_key("fast_routing"));

        let heavy_context = router.strategies.get("heavy_context").unwrap();
        assert_eq!(heavy_context.primary, "google/gemini-2.0-flash");
        assert!(!heavy_context.fallbacks.is_empty());
        assert!(heavy_context.fallbacks.len() >= 1);

        let reasoning = router.strategies.get("reasoning").unwrap();
        assert_eq!(reasoning.primary, "openrouter/free");
        assert!(!reasoning.fallbacks.is_empty());

        let fast_routing = router.strategies.get("fast_routing").unwrap();
        assert_eq!(fast_routing.primary, "groq/llama-3.3-70b-versatile");
        assert!(!fast_routing.fallbacks.is_empty());
    }

    #[tokio::test]
    async fn test_parse_provider_model() {
        let router = LlmRouter::new();

        // Test Google provider
        let config = router
            .parse_provider_model("google/gemini-2.0-flash")
            .unwrap();
        assert_eq!(config.provider, LlmProvider::Google);
        assert_eq!(config.model, Some("gemini-2.0-flash".to_string()));

        // Test OpenRouter provider
        let config = router
            .parse_provider_model("openrouter/mistralai/mistral-nemo")
            .unwrap();
        assert_eq!(config.provider, LlmProvider::Openrouter);
        assert_eq!(config.model, Some("mistralai/mistral-nemo".to_string()));

        // Test Groq provider (custom)
        let config = router
            .parse_provider_model("groq/llama-3.3-70b-versatile")
            .unwrap();
        assert_eq!(config.provider, LlmProvider::Custom);
        assert_eq!(
            config.base_url,
            Some("https://api.groq.com/openai/v1".to_string())
        );
        assert_eq!(config.model, Some("llama-3.3-70b-versatile".to_string()));

        // Test GitHub provider
        let config = router.parse_provider_model("github/gpt-4o-mini").unwrap();
        assert_eq!(config.provider, LlmProvider::Openai);
        assert_eq!(
            config.base_url,
            Some("https://models.inference.ai/v1".to_string())
        );
        assert_eq!(config.model, Some("gpt-4o-mini".to_string()));

        // Test invalid format
        assert!(router.parse_provider_model("invalid").is_err());
        assert!(
            router
                .parse_provider_model("unknown/provider/model")
                .is_err()
        );
    }

    #[test]
    fn test_strategy_config() {
        let config = StrategyConfig {
            primary: "google/gemini-2.0-flash".to_string(),
            fallbacks: vec![
                "openrouter/google/gemini-2.0-flash:free".to_string(),
                "github/gpt-4o-mini".to_string(),
            ],
        };

        assert_eq!(config.primary, "google/gemini-2.0-flash");
        assert_eq!(config.fallbacks.len(), 2);
    }

    // Integration tests
    #[tokio::test]
    async fn test_router_backward_compatibility() {
        // Test that pipelines without strategy still work (backward compatible)
        let pipeline_json = serde_json::json!({
            "id": "test-pipeline",
            "name": "Test Pipeline",
            "strategy": "seo_blog",
            "config_json": {
                "llm": {
                    "provider": "google",
                    "model": "gemini-2.0-flash"
                }
            }
        });

        let pipeline = DaemonPipeline {
            id: "test-pipeline".to_string(),
            name: "Test Pipeline".to_string(),
            strategy: "seo_blog".to_string(),
            config_json: pipeline_json.to_string(),
            schedule_cron: "0 3 * * *".to_string(),
            enabled: true,
            max_retries: 3,
            retry_delay_sec: 300,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        // Should create a single-provider client (backward compatible)
        let result = LlmRouter::from_pipeline_config(&pipeline).await;
        // This will fail if API keys are not set, but that's expected
        // We're just testing that it doesn't panic and returns the right type
        if let Err(e) = &result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("API key") || err_msg.contains("No API key"));
        }
        // If it succeeds, that's also fine (means API keys are set)
    }

    #[tokio::test]
    async fn test_router_with_strategy() {
        // Test that pipelines with strategy create a router client
        let pipeline_json = serde_json::json!({
            "id": "test-pipeline-strategy",
            "name": "Test Pipeline Strategy",
            "strategy": "seo_blog",
            "config_json": {
                "llm": {
                    "strategy": "heavy_context"
                }
            }
        });

        let pipeline = DaemonPipeline {
            id: "test-pipeline-strategy".to_string(),
            name: "Test Pipeline Strategy".to_string(),
            strategy: "seo_blog".to_string(),
            config_json: pipeline_json.to_string(),
            schedule_cron: "0 3 * * *".to_string(),
            enabled: true,
            max_retries: 3,
            retry_delay_sec: 300,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        // Should create a strategy client
        let result = LlmRouter::from_pipeline_config(&pipeline).await;
        // This will fail if API keys are not set, but that's expected
        // We're just testing that it doesn't panic and recognizes the strategy
        if let Err(e) = &result {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("API key")
                    || err_msg.contains("No API key")
                    || err_msg.contains("Unknown strategy")
            );
        }
        // If it succeeds, that's also fine (means API keys are set)
    }

    #[tokio::test]
    async fn test_circuit_breaker_integration() {
        let router = LlmRouter::new();

        // Test that circuit breaker state is shared across router instances
        let identifier = "test-provider/model";

        // Initially should allow
        assert!(router.can_attempt_provider(identifier).await);

        // Record failures
        router.record_provider_failure(identifier).await;
        router.record_provider_failure(identifier).await;
        assert!(router.can_attempt_provider(identifier).await);

        // Third failure should open circuit
        router.record_provider_failure(identifier).await;
        assert!(!router.can_attempt_provider(identifier).await);

        // Record success should close circuit
        router.record_provider_success(identifier).await;
        assert!(router.can_attempt_provider(identifier).await);
    }

    #[tokio::test]
    async fn test_router_clone() {
        // Test that router can be cloned (needed for StrategyClient)
        let router1 = LlmRouter::new();
        let router2 = router1.clone();

        assert_eq!(router1.strategies.len(), router2.strategies.len());
        assert_eq!(
            router1.strategies.keys().count(),
            router2.strategies.keys().count()
        );
    }
}
