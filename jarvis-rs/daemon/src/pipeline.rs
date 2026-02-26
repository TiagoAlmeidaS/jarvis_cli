//! Pipeline trait and registry.
//!
//! Each automation strategy (SEO blog, YouTube shorts, SaaS API) implements
//! the [`Pipeline`] trait. The [`PipelineRegistry`] maps strategy names to
//! their concrete implementations.

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::ContentOutput;
use jarvis_daemon_common::DaemonJob;
use jarvis_daemon_common::DaemonPipeline;
use jarvis_daemon_common::DaemonSource;
use jarvis_daemon_common::LogLevel;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::processor::LlmClient;
use jarvis_daemon_common::DaemonDb;

/// Context passed to a pipeline during execution.
pub struct PipelineContext {
    /// The job being executed.
    pub job: DaemonJob,
    /// The pipeline definition.
    pub pipeline: DaemonPipeline,
    /// Sources registered for this pipeline.
    pub sources: Vec<DaemonSource>,
    /// LLM client for text generation.
    pub llm_client: Arc<dyn LlmClient>,
    /// Database handle for logging and state updates.
    pub db: Arc<DaemonDb>,
    /// Cancellation token for graceful shutdown.
    pub cancellation_token: CancellationToken,
}

impl PipelineContext {
    /// Log a message to the daemon_logs table.
    pub async fn log(&self, level: LogLevel, message: &str) {
        if let Err(e) = self
            .db
            .insert_log(&self.pipeline.id, Some(&self.job.id), level, message, None)
            .await
        {
            tracing::warn!("Failed to insert daemon log: {e}");
        }
    }

    /// Log an info message.
    pub async fn log_info(&self, message: &str) {
        self.log(LogLevel::Info, message).await;
    }

    /// Log an error message.
    pub async fn log_error(&self, message: &str) {
        self.log(LogLevel::Error, message).await;
    }
}

/// Trait that every automation pipeline must implement.
#[async_trait]
pub trait Pipeline: Send + Sync {
    /// The strategy identifier (matches [`Strategy`] enum values).
    fn strategy(&self) -> &str;

    /// Human-readable name for logs and UI.
    fn display_name(&self) -> &str;

    /// Validate pipeline configuration before execution.
    async fn validate_config(&self, config: &serde_json::Value) -> Result<()>;

    /// Execute the pipeline and return generated content.
    async fn execute(&self, ctx: &PipelineContext) -> Result<Vec<ContentOutput>>;
}

/// Registry that maps strategy names to their pipeline implementations.
pub struct PipelineRegistry {
    pipelines: HashMap<String, Arc<dyn Pipeline>>,
}

impl PipelineRegistry {
    /// Create a new registry with all built-in pipeline implementations.
    pub fn new() -> Self {
        let mut pipelines: HashMap<String, Arc<dyn Pipeline>> = HashMap::new();

        // Register the SEO blog pipeline.
        let seo = Arc::new(crate::pipelines::seo_blog::SeoBlogPipeline);
        pipelines.insert(seo.strategy().to_string(), seo);

        // Register the metrics collector pipeline.
        let metrics = Arc::new(crate::pipelines::metrics_collector::MetricsCollectorPipeline);
        pipelines.insert(metrics.strategy().to_string(), metrics);

        // Register the strategy analyzer pipeline.
        let analyzer = Arc::new(crate::pipelines::strategy_analyzer::StrategyAnalyzerPipeline);
        pipelines.insert(analyzer.strategy().to_string(), analyzer);

        // Register the A/B tester pipeline.
        let ab_tester = Arc::new(crate::pipelines::ab_tester::AbTesterPipeline);
        pipelines.insert(ab_tester.strategy().to_string(), ab_tester);

        // Register the prompt optimizer pipeline.
        let prompt_opt = Arc::new(crate::pipelines::prompt_optimizer::PromptOptimizerPipeline);
        pipelines.insert(prompt_opt.strategy().to_string(), prompt_opt);

        Self { pipelines }
    }

    /// Get a pipeline implementation by strategy name.
    pub fn get(&self, strategy: &str) -> Option<Arc<dyn Pipeline>> {
        self.pipelines.get(strategy).cloned()
    }

    /// List all registered strategy names.
    pub fn strategies(&self) -> Vec<&str> {
        self.pipelines.keys().map(String::as_str).collect()
    }
}
