//! Cron-like scheduler that ticks every interval, checks which pipelines
//! are due for execution, and enqueues jobs.

use anyhow::Result;
use jarvis_daemon_common::CronSchedule;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::LogLevel;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::executor::ProposalExecutor;
use crate::runner::PipelineRunner;

/// The scheduler loop.
pub struct Scheduler {
    db: Arc<DaemonDb>,
    runner: Arc<PipelineRunner>,
    executor: ProposalExecutor,
    tick_interval: Duration,
    shutdown: CancellationToken,
}

impl Scheduler {
    pub fn new(
        db: Arc<DaemonDb>,
        runner: Arc<PipelineRunner>,
        tick_interval: Duration,
        shutdown: CancellationToken,
    ) -> Self {
        let executor = ProposalExecutor::new(db.clone());
        Self {
            db,
            runner,
            executor,
            tick_interval,
            shutdown,
        }
    }

    /// Run the scheduler loop until shutdown is signalled.
    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(self.tick_interval);
        // Don't burst-fire missed ticks after a long pipeline run.
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.tick().await {
                        error!("Scheduler tick error: {e:#}");
                    }
                }
                _ = self.shutdown.cancelled() => {
                    info!("Scheduler shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// One scheduler tick: check all enabled pipelines and enqueue jobs if
    /// their cron schedule matches the current time.
    async fn tick(&self) -> Result<()> {
        debug!("Scheduler tick");

        let pipelines = self.db.list_pipelines(true).await?;

        for pipeline in &pipelines {
            let schedule = match CronSchedule::parse(&pipeline.schedule_cron) {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        "Pipeline '{}' has invalid cron '{}': {e}",
                        pipeline.id, pipeline.schedule_cron
                    );
                    continue;
                }
            };

            if !schedule.matches_now() {
                continue;
            }

            // Check if there's already a running or pending job for this pipeline
            // to avoid duplicate executions within the same cron window.
            let filter = jarvis_daemon_common::JobFilter {
                pipeline_id: Some(pipeline.id.clone()),
                status: Some(jarvis_daemon_common::JobStatus::Running),
                limit: Some(1),
            };
            let running = self.db.list_jobs(&filter).await?;
            if !running.is_empty() {
                debug!(
                    "Pipeline '{}' already has a running job, skipping",
                    pipeline.id
                );
                continue;
            }

            let pending_filter = jarvis_daemon_common::JobFilter {
                pipeline_id: Some(pipeline.id.clone()),
                status: Some(jarvis_daemon_common::JobStatus::Pending),
                limit: Some(1),
            };
            let pending = self.db.list_jobs(&pending_filter).await?;
            if !pending.is_empty() {
                debug!(
                    "Pipeline '{}' already has a pending job, skipping",
                    pipeline.id
                );
                continue;
            }

            info!(
                "Schedule matched for pipeline '{}' ({}), creating job",
                pipeline.id, pipeline.schedule_cron
            );

            let _ = self
                .db
                .insert_log(
                    &pipeline.id,
                    None,
                    LogLevel::Info,
                    &format!(
                        "Schedule '{}' matched, enqueuing job",
                        pipeline.schedule_cron
                    ),
                    None,
                )
                .await;

            // Create the job.
            match self.db.create_job(&pipeline.id).await {
                Ok(job) => {
                    // Spawn execution in the runner.
                    let runner = self.runner.clone();
                    let pipeline_clone = pipeline.clone();
                    let job_id = job.id.clone();
                    tokio::spawn(async move {
                        if let Err(e) = runner.run_job(pipeline_clone, job).await {
                            error!("Job {job_id} failed: {e:#}");
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to create job for pipeline '{}': {e}", pipeline.id);
                }
            }
        }

        // Execute approved proposals.
        match self.executor.execute_pending().await {
            Ok(summary) => {
                if summary.executed > 0 || summary.failed > 0 {
                    info!(
                        "Proposal execution: {} executed, {} failed, {} skipped",
                        summary.executed, summary.failed, summary.skipped
                    );
                }
            }
            Err(e) => {
                error!("Proposal executor error: {e:#}");
            }
        }

        Ok(())
    }
}
