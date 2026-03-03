//! Pipeline runner — orchestrates the execution of a single pipeline job.
//!
//! The runner:
//! 1. Checks concurrency limits
//! 2. Marks the job as running
//! 3. Loads sources for the pipeline
//! 4. Delegates to the pipeline implementation
//! 5. Persists generated content
//! 6. Retries on failure (up to `max_retries` with `retry_delay_sec` backoff)

use anyhow::Result;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::DaemonJob;
use jarvis_daemon_common::DaemonPipeline;
use jarvis_daemon_common::LogLevel;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::pipeline::PipelineContext;
use crate::pipeline::PipelineRegistry;
use crate::processor::router::LlmRouter;

/// Orchestrates pipeline job execution.
pub struct PipelineRunner {
    db: Arc<DaemonDb>,
    registry: Arc<PipelineRegistry>,
    max_concurrent: usize,
}

impl PipelineRunner {
    pub fn new(db: Arc<DaemonDb>, registry: Arc<PipelineRegistry>, max_concurrent: usize) -> Self {
        Self {
            db,
            registry,
            max_concurrent,
        }
    }

    /// Execute a single job for the given pipeline.
    pub async fn run_job(&self, pipeline: DaemonPipeline, job: DaemonJob) -> Result<()> {
        let job_id = job.id.clone();
        let pipeline_id = pipeline.id.clone();

        // Check concurrency.
        let running = self.db.count_running_jobs().await?;
        if running >= self.max_concurrent as i64 {
            info!(
                "Concurrency limit reached ({}/{}), deferring job {}",
                running, self.max_concurrent, job_id
            );
            return Ok(());
        }

        // Get the pipeline implementation.
        let impl_ = self
            .registry
            .get(&pipeline.strategy)
            .ok_or_else(|| anyhow::anyhow!("no pipeline for strategy '{}'", pipeline.strategy))?;

        // Mark job as running.
        self.db.start_job(&job_id).await?;
        self.db
            .insert_log(
                &pipeline_id,
                Some(&job_id),
                LogLevel::Info,
                &format!("Job started: {job_id}"),
                None,
            )
            .await?;

        // Load sources.
        let sources = self.db.list_sources(&pipeline_id).await?;

        // Build LLM client from pipeline config (with fallback support if strategy is configured).
        let llm_client = LlmRouter::from_pipeline_config(&pipeline).await?;

        // Build context.
        let ctx = PipelineContext {
            job,
            pipeline: pipeline.clone(),
            sources,
            llm_client,
            db: self.db.clone(),
            cancellation_token: CancellationToken::new(),
        };

        // Execute.
        match impl_.execute(&ctx).await {
            Ok(outputs) => {
                info!(
                    "Pipeline '{}' job {} produced {} content items",
                    pipeline_id,
                    job_id,
                    outputs.len()
                );

                // Persist each content output.
                for output in &outputs {
                    match self.db.create_content(&job_id, &pipeline_id, output).await {
                        Ok(content) => {
                            info!(
                                "Content created: {} ({} on {})",
                                content.id, output.title, output.platform
                            );
                        }
                        Err(e) => {
                            error!("Failed to persist content: {e}");
                        }
                    }
                }

                // Mark job as completed.
                let output_summary =
                    serde_json::json!({"content_count": outputs.len()}).to_string();
                self.db.complete_job(&job_id, Some(&output_summary)).await?;
                self.db
                    .insert_log(
                        &pipeline_id,
                        Some(&job_id),
                        LogLevel::Info,
                        &format!(
                            "Job completed successfully: {} content items",
                            outputs.len()
                        ),
                        None,
                    )
                    .await?;
            }
            Err(e) => {
                let err_msg = format!("{e:#}");
                error!(
                    "Pipeline '{}' job {} failed: {err_msg}",
                    pipeline_id, job_id
                );

                self.db
                    .fail_job(&job_id, &err_msg, Some(&format!("{e:?}")))
                    .await?;
                self.db
                    .insert_log(
                        &pipeline_id,
                        Some(&job_id),
                        LogLevel::Error,
                        &format!("Job failed: {err_msg}"),
                        None,
                    )
                    .await?;

                // Retry logic: create a new pending job if below max_retries.
                // The scheduler will pick it up on the next tick, naturally
                // respecting retry_delay_sec via the job's created_at timestamp.
                let attempt = ctx.job.attempt;
                let max_retries = pipeline.max_retries;
                if attempt < max_retries {
                    let next_attempt = attempt + 1;
                    warn!(
                        "Scheduling retry {next_attempt}/{max_retries} for pipeline '{}'",
                        pipeline_id,
                    );

                    match self
                        .db
                        .create_job_with_attempt(&pipeline_id, next_attempt)
                        .await
                    {
                        Ok(retry_job) => {
                            self.db
                                .insert_log(
                                    &pipeline_id,
                                    Some(&retry_job.id),
                                    LogLevel::Info,
                                    &format!(
                                        "Retry {next_attempt}/{max_retries} enqueued (from job {job_id})"
                                    ),
                                    None,
                                )
                                .await?;
                        }
                        Err(e) => {
                            error!("Failed to create retry job: {e:#}");
                        }
                    }
                } else {
                    warn!(
                        "Pipeline '{}' job {} exhausted all {max_retries} retries",
                        pipeline_id, job_id,
                    );
                    self.db
                        .insert_log(
                            &pipeline_id,
                            Some(&job_id),
                            LogLevel::Error,
                            &format!("All {max_retries} retries exhausted"),
                            None,
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }
}
