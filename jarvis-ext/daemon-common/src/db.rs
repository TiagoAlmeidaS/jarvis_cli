//! SQLite database operations for the daemon automation system.
//!
//! Provides CRUD operations for all daemon tables, using `sqlx` with
//! the same SQLite backend as `jarvis-state`.

use crate::models::*;
use anyhow::Result;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

/// Database handle for daemon operations.
#[derive(Debug, Clone)]
pub struct DaemonDb {
    pool: SqlitePool,
}

impl DaemonDb {
    /// Open (or create) the daemon database at the given path.
    pub async fn open(path: &Path) -> Result<Self> {
        let db_url = format!("sqlite:{}?mode=rwc", path.display());
        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    #[cfg(any(test, feature = "test-support"))]
    pub async fn open_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Run the daemon schema migrations.
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(MIGRATION_SQL).execute(&self.pool).await?;
        Ok(())
    }

    /// Get a reference to the underlying pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // -----------------------------------------------------------------------
    // Pipelines
    // -----------------------------------------------------------------------

    /// Insert a new pipeline.
    pub async fn create_pipeline(&self, input: &CreatePipeline) -> Result<DaemonPipeline> {
        let now = chrono::Utc::now().timestamp();
        let config_str = serde_json::to_string(&input.config_json)?;
        let strategy_str = input.strategy.to_string();
        let max_retries = input.max_retries.unwrap_or(3);
        let retry_delay = input.retry_delay_sec.unwrap_or(300);

        sqlx::query(
            "INSERT INTO daemon_pipelines (id, name, strategy, config_json, schedule_cron, enabled, max_retries, retry_delay_sec, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?)",
        )
        .bind(&input.id)
        .bind(&input.name)
        .bind(&strategy_str)
        .bind(&config_str)
        .bind(&input.schedule_cron)
        .bind(max_retries)
        .bind(retry_delay)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_pipeline(&input.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("pipeline not found after insert"))
    }

    /// Get a pipeline by ID.
    pub async fn get_pipeline(&self, id: &str) -> Result<Option<DaemonPipeline>> {
        let row = sqlx::query_as::<_, PipelineRow>(
            "SELECT id, name, strategy, config_json, schedule_cron, enabled, max_retries, retry_delay_sec, created_at, updated_at
             FROM daemon_pipelines WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    /// List all pipelines, optionally filtered by enabled status.
    pub async fn list_pipelines(&self, enabled_only: bool) -> Result<Vec<DaemonPipeline>> {
        let rows = if enabled_only {
            sqlx::query_as::<_, PipelineRow>(
                "SELECT id, name, strategy, config_json, schedule_cron, enabled, max_retries, retry_delay_sec, created_at, updated_at
                 FROM daemon_pipelines WHERE enabled = 1 ORDER BY created_at DESC",
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PipelineRow>(
                "SELECT id, name, strategy, config_json, schedule_cron, enabled, max_retries, retry_delay_sec, created_at, updated_at
                 FROM daemon_pipelines ORDER BY created_at DESC",
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Enable or disable a pipeline.
    pub async fn set_pipeline_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE daemon_pipelines SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Jobs
    // -----------------------------------------------------------------------

    /// Insert a new job.
    pub async fn create_job(&self, pipeline_id: &str) -> Result<DaemonJob> {
        let job = DaemonJob::new(pipeline_id);
        sqlx::query(
            "INSERT INTO daemon_jobs (id, pipeline_id, status, attempt, started_at, completed_at, input_json, output_json, error_message, error_stack, duration_ms, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&job.id)
        .bind(&job.pipeline_id)
        .bind(&job.status)
        .bind(job.attempt)
        .bind(job.started_at)
        .bind(job.completed_at)
        .bind(&job.input_json)
        .bind(&job.output_json)
        .bind(&job.error_message)
        .bind(&job.error_stack)
        .bind(job.duration_ms)
        .bind(job.created_at)
        .execute(&self.pool)
        .await?;

        Ok(job)
    }

    /// Create a job with a specific attempt number (used for retries).
    pub async fn create_job_with_attempt(
        &self,
        pipeline_id: &str,
        attempt: i32,
    ) -> Result<DaemonJob> {
        let mut job = DaemonJob::new(pipeline_id);
        job.attempt = attempt;
        sqlx::query(
            "INSERT INTO daemon_jobs (id, pipeline_id, status, attempt, started_at, completed_at, input_json, output_json, error_message, error_stack, duration_ms, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&job.id)
        .bind(&job.pipeline_id)
        .bind(&job.status)
        .bind(job.attempt)
        .bind(job.started_at)
        .bind(job.completed_at)
        .bind(&job.input_json)
        .bind(&job.output_json)
        .bind(&job.error_message)
        .bind(&job.error_stack)
        .bind(job.duration_ms)
        .bind(job.created_at)
        .execute(&self.pool)
        .await?;

        Ok(job)
    }

    /// Mark a job as running.
    pub async fn start_job(&self, job_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE daemon_jobs SET status = 'running', started_at = ? WHERE id = ?")
            .bind(now)
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark a job as completed with output.
    pub async fn complete_job(&self, job_id: &str, output_json: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        // Calculate duration from started_at
        let row = sqlx::query("SELECT started_at FROM daemon_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await?;

        let duration_ms = row
            .and_then(|r| r.get::<Option<i64>, _>("started_at"))
            .map(|started| (now - started) * 1000);

        sqlx::query(
            "UPDATE daemon_jobs SET status = 'completed', completed_at = ?, output_json = ?, duration_ms = ? WHERE id = ?",
        )
        .bind(now)
        .bind(output_json)
        .bind(duration_ms)
        .bind(job_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark a job as failed with error info.
    pub async fn fail_job(
        &self,
        job_id: &str,
        error_message: &str,
        error_stack: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let row = sqlx::query("SELECT started_at FROM daemon_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await?;

        let duration_ms = row
            .and_then(|r| r.get::<Option<i64>, _>("started_at"))
            .map(|started| (now - started) * 1000);

        sqlx::query(
            "UPDATE daemon_jobs SET status = 'failed', completed_at = ?, error_message = ?, error_stack = ?, duration_ms = ? WHERE id = ?",
        )
        .bind(now)
        .bind(error_message)
        .bind(error_stack)
        .bind(duration_ms)
        .bind(job_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List jobs with optional filtering.
    pub async fn list_jobs(&self, filter: &JobFilter) -> Result<Vec<DaemonJob>> {
        let limit = filter.limit.unwrap_or(50);
        let mut query = String::from(
            "SELECT id, pipeline_id, status, attempt, started_at, completed_at, input_json, output_json, error_message, error_stack, duration_ms, created_at FROM daemon_jobs WHERE 1=1",
        );

        if let Some(ref pid) = filter.pipeline_id {
            query.push_str(&format!(" AND pipeline_id = '{pid}'"));
        }
        if let Some(ref status) = filter.status {
            query.push_str(&format!(" AND status = '{status}'"));
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));

        let rows = sqlx::query_as::<_, JobRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Count running jobs for concurrency control.
    pub async fn count_running_jobs(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM daemon_jobs WHERE status = 'running'")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("cnt"))
    }

    // -----------------------------------------------------------------------
    // Content
    // -----------------------------------------------------------------------

    /// Insert content produced by a pipeline.
    pub async fn create_content(
        &self,
        job_id: &str,
        pipeline_id: &str,
        output: &ContentOutput,
    ) -> Result<DaemonContent> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let content_type = output.content_type.to_string();
        let platform = output.platform.to_string();
        let content_hash = compute_content_hash(&output.body);

        sqlx::query(
            "INSERT INTO daemon_content (id, job_id, pipeline_id, content_type, platform, title, slug, url, status, word_count, llm_model, llm_tokens_used, llm_cost_usd, content_hash, created_at, published_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(&id)
        .bind(job_id)
        .bind(pipeline_id)
        .bind(&content_type)
        .bind(&platform)
        .bind(&output.title)
        .bind(&output.slug)
        .bind(&output.url)
        .bind(output.word_count)
        .bind(&output.llm_model)
        .bind(output.llm_tokens_used)
        .bind(output.llm_cost_usd)
        .bind(&content_hash)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(DaemonContent {
            id,
            job_id: job_id.to_string(),
            pipeline_id: pipeline_id.to_string(),
            content_type,
            platform,
            title: Some(output.title.clone()),
            slug: Some(output.slug.clone()),
            url: output.url.clone(),
            status: ContentStatus::Draft.to_string(),
            word_count: output.word_count,
            llm_model: Some(output.llm_model.clone()),
            llm_tokens_used: Some(output.llm_tokens_used),
            llm_cost_usd: output.llm_cost_usd,
            content_hash: Some(content_hash),
            created_at: now,
            published_at: None,
        })
    }

    /// Mark content as published.
    pub async fn publish_content(&self, content_id: &str, url: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE daemon_content SET status = 'published', url = ?, published_at = ? WHERE id = ?")
            .bind(url)
            .bind(now)
            .bind(content_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Check if content with this hash already exists (deduplication).
    pub async fn content_exists_by_hash(&self, hash: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM daemon_content WHERE content_hash = ?")
            .bind(hash)
            .fetch_one(&self.pool)
            .await?;
        let count: i64 = row.get("cnt");
        Ok(count > 0)
    }

    /// List content with optional filtering.
    pub async fn list_content(&self, filter: &ContentFilter) -> Result<Vec<DaemonContent>> {
        let limit = filter.limit.unwrap_or(50);
        let mut query = String::from(
            "SELECT id, job_id, pipeline_id, content_type, platform, title, slug, url, status, word_count, llm_model, llm_tokens_used, llm_cost_usd, content_hash, created_at, published_at FROM daemon_content WHERE 1=1",
        );

        if let Some(ref pid) = filter.pipeline_id {
            query.push_str(&format!(" AND pipeline_id = '{pid}'"));
        }
        if let Some(ref status) = filter.status {
            query.push_str(&format!(" AND status = '{status}'"));
        }
        if let Some(days) = filter.since_days {
            let since = chrono::Utc::now().timestamp() - (days * 86400);
            query.push_str(&format!(" AND created_at >= {since}"));
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));

        let rows = sqlx::query_as::<_, ContentRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Find a single content record by its published URL.
    pub async fn find_content_by_url(&self, url: &str) -> Result<Option<DaemonContent>> {
        let row = sqlx::query_as::<_, ContentRow>(
            "SELECT id, job_id, pipeline_id, content_type, platform, title, slug, url, status, word_count, llm_model, llm_tokens_used, llm_cost_usd, content_hash, created_at, published_at \
             FROM daemon_content WHERE url = ? LIMIT 1",
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    /// Find a single content record by its slug.
    pub async fn find_content_by_slug(&self, slug: &str) -> Result<Option<DaemonContent>> {
        let row = sqlx::query_as::<_, ContentRow>(
            "SELECT id, job_id, pipeline_id, content_type, platform, title, slug, url, status, word_count, llm_model, llm_tokens_used, llm_cost_usd, content_hash, created_at, published_at \
             FROM daemon_content WHERE slug = ? LIMIT 1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    // -----------------------------------------------------------------------
    // Sources
    // -----------------------------------------------------------------------

    /// Add a source to a pipeline.
    pub async fn create_source(&self, input: &CreateSource) -> Result<DaemonSource> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let source_type = input.source_type.to_string();
        let interval = input.check_interval_sec.unwrap_or(86400);

        sqlx::query(
            "INSERT INTO daemon_sources (id, pipeline_id, source_type, name, url, scrape_selector, last_checked_at, last_content_hash, check_interval_sec, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, ?, 1, ?, ?)",
        )
        .bind(&id)
        .bind(&input.pipeline_id)
        .bind(&source_type)
        .bind(&input.name)
        .bind(&input.url)
        .bind(&input.scrape_selector)
        .bind(interval)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(DaemonSource {
            id,
            pipeline_id: input.pipeline_id.clone(),
            source_type,
            name: input.name.clone(),
            url: input.url.clone(),
            scrape_selector: input.scrape_selector.clone(),
            last_checked_at: None,
            last_content_hash: None,
            check_interval_sec: interval,
            enabled: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// List sources for a pipeline.
    pub async fn list_sources(&self, pipeline_id: &str) -> Result<Vec<DaemonSource>> {
        let rows = sqlx::query_as::<_, SourceRow>(
            "SELECT id, pipeline_id, source_type, name, url, scrape_selector, last_checked_at, last_content_hash, check_interval_sec, enabled, created_at, updated_at
             FROM daemon_sources WHERE pipeline_id = ? ORDER BY created_at",
        )
        .bind(pipeline_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Update source after a check.
    pub async fn update_source_checked(&self, source_id: &str, content_hash: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_sources SET last_checked_at = ?, last_content_hash = ?, updated_at = ? WHERE id = ?",
        )
        .bind(now)
        .bind(content_hash)
        .bind(now)
        .bind(source_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get sources that are due for checking.
    pub async fn get_sources_due_for_check(&self, pipeline_id: &str) -> Result<Vec<DaemonSource>> {
        let now = chrono::Utc::now().timestamp();
        let rows = sqlx::query_as::<_, SourceRow>(
            "SELECT id, pipeline_id, source_type, name, url, scrape_selector, last_checked_at, last_content_hash, check_interval_sec, enabled, created_at, updated_at
             FROM daemon_sources
             WHERE pipeline_id = ? AND enabled = 1
             AND (last_checked_at IS NULL OR (? - last_checked_at) >= check_interval_sec)
             ORDER BY last_checked_at ASC NULLS FIRST",
        )
        .bind(pipeline_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    // -----------------------------------------------------------------------
    // Logs
    // -----------------------------------------------------------------------

    /// Insert a log entry.
    pub async fn insert_log(
        &self,
        pipeline_id: &str,
        job_id: Option<&str>,
        level: LogLevel,
        message: &str,
        context: Option<&serde_json::Value>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let level_str = match level {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        let ctx_str = context.map(std::string::ToString::to_string);

        sqlx::query(
            "INSERT INTO daemon_logs (job_id, pipeline_id, level, message, context_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(job_id)
        .bind(pipeline_id)
        .bind(level_str)
        .bind(message)
        .bind(ctx_str)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List log entries with optional filtering.
    pub async fn list_logs(&self, filter: &LogFilter) -> Result<Vec<DaemonLog>> {
        let limit = filter.limit.unwrap_or(100);
        let mut query = String::from(
            "SELECT id, job_id, pipeline_id, level, message, context_json, created_at FROM daemon_logs WHERE 1=1",
        );

        if let Some(ref pid) = filter.pipeline_id {
            query.push_str(&format!(" AND pipeline_id = '{pid}'"));
        }
        if let Some(ref jid) = filter.job_id {
            query.push_str(&format!(" AND job_id = '{jid}'"));
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));

        let rows = sqlx::query_as::<_, LogRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    // -----------------------------------------------------------------------
    // Metrics
    // -----------------------------------------------------------------------

    /// Insert a metric.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_metric(
        &self,
        pipeline_id: &str,
        content_id: Option<&str>,
        metric_type: MetricType,
        value: f64,
        source: &str,
        period_start: i64,
        period_end: i64,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let mt = match metric_type {
            MetricType::Views => "views",
            MetricType::Clicks => "clicks",
            MetricType::Revenue => "revenue",
            MetricType::Impressions => "impressions",
            MetricType::Subscribers => "subscribers",
            MetricType::Ctr => "ctr",
        };

        sqlx::query(
            "INSERT INTO daemon_metrics (id, content_id, pipeline_id, metric_type, value, currency, period_start, period_end, source, created_at)
             VALUES (?, ?, ?, ?, ?, 'USD', ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(content_id)
        .bind(pipeline_id)
        .bind(mt)
        .bind(value)
        .bind(period_start)
        .bind(period_end)
        .bind(source)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Sum metric values of a given type within a time window.
    ///
    /// Optionally filter by `source` (e.g. `"wordpress_stats"`) to distinguish
    /// real data from estimates.
    pub async fn sum_metrics(
        &self,
        metric_type: MetricType,
        since: i64,
        source: Option<&str>,
    ) -> Result<f64> {
        let mt = match metric_type {
            MetricType::Views => "views",
            MetricType::Clicks => "clicks",
            MetricType::Revenue => "revenue",
            MetricType::Impressions => "impressions",
            MetricType::Subscribers => "subscribers",
            MetricType::Ctr => "ctr",
        };

        let total: (f64,) = if let Some(src) = source {
            sqlx::query_as(
                "SELECT COALESCE(SUM(value), 0.0) FROM daemon_metrics WHERE metric_type = ? AND period_end >= ? AND source = ?",
            )
            .bind(mt)
            .bind(since)
            .bind(src)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT COALESCE(SUM(value), 0.0) FROM daemon_metrics WHERE metric_type = ? AND period_end >= ?",
            )
            .bind(mt)
            .bind(since)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(total.0)
    }

    /// Sum a specific metric for a given content_id.
    pub async fn sum_content_metric(&self, content_id: &str, metric_type: &str) -> Result<f64> {
        let total: (f64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(value), 0.0) FROM daemon_metrics \
             WHERE content_id = ?1 AND metric_type = ?2",
        )
        .bind(content_id)
        .bind(metric_type)
        .fetch_one(&self.pool)
        .await?;
        Ok(total.0)
    }

    // -----------------------------------------------------------------------
    // Proposals
    // -----------------------------------------------------------------------

    /// Create a new proposal.
    pub async fn create_proposal(&self, input: &CreateProposal) -> Result<DaemonProposal> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let action_type = input.action_type.to_string();
        let risk_level = input.risk_level.to_string();
        let proposed_config = input
            .proposed_config
            .as_ref()
            .map(std::string::ToString::to_string);
        let metrics_snapshot = input
            .metrics_snapshot
            .as_ref()
            .map(std::string::ToString::to_string);
        let expires_at = input.expires_in_hours.map(|h| now + h * 3600);

        sqlx::query(
            "INSERT INTO daemon_proposals (id, pipeline_id, action_type, title, description, reasoning, confidence, risk_level, status, proposed_config, metrics_snapshot, auto_approvable, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&input.pipeline_id)
        .bind(&action_type)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.reasoning)
        .bind(input.confidence)
        .bind(&risk_level)
        .bind(&proposed_config)
        .bind(&metrics_snapshot)
        .bind(input.auto_approvable)
        .bind(now)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(DaemonProposal {
            id,
            pipeline_id: input.pipeline_id.clone(),
            action_type,
            title: input.title.clone(),
            description: input.description.clone(),
            reasoning: input.reasoning.clone(),
            confidence: input.confidence,
            risk_level,
            status: ProposalStatus::Pending.to_string(),
            proposed_config,
            metrics_snapshot,
            auto_approvable: input.auto_approvable,
            created_at: now,
            reviewed_at: None,
            executed_at: None,
            expires_at,
        })
    }

    /// Get a single proposal by ID.
    pub async fn get_proposal(&self, id: &str) -> Result<Option<DaemonProposal>> {
        let row = sqlx::query_as::<_, ProposalRow>(
            "SELECT id, pipeline_id, action_type, title, description, reasoning, confidence, risk_level, status, proposed_config, metrics_snapshot, auto_approvable, created_at, reviewed_at, executed_at, expires_at
             FROM daemon_proposals WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    /// List proposals with optional filtering.
    pub async fn list_proposals(&self, filter: &ProposalFilter) -> Result<Vec<DaemonProposal>> {
        let limit = filter.limit.unwrap_or(50);
        let mut query = String::from(
            "SELECT id, pipeline_id, action_type, title, description, reasoning, confidence, risk_level, status, proposed_config, metrics_snapshot, auto_approvable, created_at, reviewed_at, executed_at, expires_at FROM daemon_proposals WHERE 1=1",
        );

        if let Some(ref pid) = filter.pipeline_id {
            query.push_str(&format!(" AND pipeline_id = '{pid}'"));
        }
        if let Some(ref status) = filter.status {
            query.push_str(&format!(" AND status = '{status}'"));
        }
        if let Some(ref risk) = filter.risk_level {
            query.push_str(&format!(" AND risk_level = '{risk}'"));
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));

        let rows = sqlx::query_as::<_, ProposalRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Approve a proposal.
    pub async fn approve_proposal(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query(
            "UPDATE daemon_proposals SET status = 'approved', reviewed_at = ? WHERE id = ? AND status = 'pending'",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("proposal {id} not found or not in pending status");
        }
        Ok(())
    }

    /// Reject a proposal.
    pub async fn reject_proposal(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query(
            "UPDATE daemon_proposals SET status = 'rejected', reviewed_at = ? WHERE id = ? AND status = 'pending'",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("proposal {id} not found or not in pending status");
        }
        Ok(())
    }

    /// Mark a proposal as executed.
    pub async fn mark_proposal_executed(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_proposals SET status = 'executed', executed_at = ? WHERE id = ? AND status = 'approved'",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark a proposal as failed during execution.
    pub async fn mark_proposal_failed(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_proposals SET status = 'failed', executed_at = ? WHERE id = ? AND status = 'approved'",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Expire proposals that have passed their expiration date.
    pub async fn expire_proposals(&self) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query(
            "UPDATE daemon_proposals SET status = 'expired' WHERE status = 'pending' AND expires_at IS NOT NULL AND expires_at < ?",
        )
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Count pending proposals.
    pub async fn count_pending_proposals(&self) -> Result<i64> {
        let row =
            sqlx::query("SELECT COUNT(*) as count FROM daemon_proposals WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await?;
        Ok(row.get::<i64, _>("count"))
    }

    // -----------------------------------------------------------------------
    // Revenue
    // -----------------------------------------------------------------------

    /// Record a revenue entry.
    pub async fn create_revenue(&self, input: &CreateRevenue) -> Result<DaemonRevenue> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let source = input.source.to_string();
        let currency = input.currency.as_deref().unwrap_or("USD").to_string();
        let metadata_json = input
            .metadata
            .as_ref()
            .map(std::string::ToString::to_string);

        sqlx::query(
            "INSERT INTO daemon_revenue (id, content_id, pipeline_id, source, amount, currency, period_start, period_end, external_id, metadata_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&input.content_id)
        .bind(&input.pipeline_id)
        .bind(&source)
        .bind(input.amount)
        .bind(&currency)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(&input.external_id)
        .bind(&metadata_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(DaemonRevenue {
            id,
            content_id: input.content_id.clone(),
            pipeline_id: input.pipeline_id.clone(),
            source,
            amount: input.amount,
            currency,
            period_start: input.period_start,
            period_end: input.period_end,
            external_id: input.external_id.clone(),
            metadata_json,
            created_at: now,
        })
    }

    /// List revenue records with optional filtering.
    pub async fn list_revenue(&self, filter: &RevenueFilter) -> Result<Vec<DaemonRevenue>> {
        let limit = filter.limit.unwrap_or(100);
        let mut query = String::from(
            "SELECT id, content_id, pipeline_id, source, amount, currency, period_start, period_end, external_id, metadata_json, created_at FROM daemon_revenue WHERE 1=1",
        );

        if let Some(ref pid) = filter.pipeline_id {
            query.push_str(&format!(" AND pipeline_id = '{pid}'"));
        }
        if let Some(ref src) = filter.source {
            query.push_str(&format!(" AND source = '{src}'"));
        }
        if let Some(days) = filter.since_days {
            let since = chrono::Utc::now().timestamp() - days * 86400;
            query.push_str(&format!(" AND period_start >= {since}"));
        }

        query.push_str(&format!(" ORDER BY period_start DESC LIMIT {limit}"));

        let rows = sqlx::query_as::<_, RevenueRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get a revenue summary for the given number of days.
    pub async fn revenue_summary(&self, days: i64) -> Result<RevenueSummary> {
        let since = chrono::Utc::now().timestamp() - days * 86400;

        // Total revenue.
        let total_row = sqlx::query(
            "SELECT COALESCE(SUM(amount), 0.0) as total FROM daemon_revenue WHERE period_start >= ?",
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await?;
        let total_usd: f64 = total_row.get("total");

        // By pipeline.
        let pipeline_rows = sqlx::query(
            "SELECT r.pipeline_id, p.name as pipeline_name, COALESCE(SUM(r.amount), 0.0) as total, COUNT(*) as cnt
             FROM daemon_revenue r
             LEFT JOIN daemon_pipelines p ON p.id = r.pipeline_id
             WHERE r.period_start >= ?
             GROUP BY r.pipeline_id
             ORDER BY total DESC",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        let by_pipeline = pipeline_rows
            .iter()
            .map(|r| PipelineRevenue {
                pipeline_id: r.get("pipeline_id"),
                pipeline_name: r.get("pipeline_name"),
                total_usd: r.get("total"),
                content_count: r.get("cnt"),
            })
            .collect();

        // By source.
        let source_rows = sqlx::query(
            "SELECT source, COALESCE(SUM(amount), 0.0) as total, COUNT(*) as cnt
             FROM daemon_revenue WHERE period_start >= ?
             GROUP BY source ORDER BY total DESC",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        let by_source = source_rows
            .iter()
            .map(|r| SourceRevenue {
                source: r.get("source"),
                total_usd: r.get("total"),
                record_count: r.get("cnt"),
            })
            .collect();

        Ok(RevenueSummary {
            total_usd,
            period_days: days,
            by_pipeline,
            by_source,
        })
    }

    // -----------------------------------------------------------------------
    // Pipeline updates (used by ProposalExecutor)
    // -----------------------------------------------------------------------

    /// Update a pipeline's config_json.
    pub async fn update_pipeline_config(&self, id: &str, config: &serde_json::Value) -> Result<()> {
        let config_str = serde_json::to_string(config)?;
        let now = chrono::Utc::now().timestamp();
        let rows = sqlx::query(
            "UPDATE daemon_pipelines SET config_json = ?1, updated_at = ?2 WHERE id = ?3",
        )
        .bind(&config_str)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if rows == 0 {
            anyhow::bail!("pipeline '{id}' not found");
        }
        Ok(())
    }

    /// Update a pipeline's schedule_cron.
    pub async fn update_pipeline_schedule(&self, id: &str, cron: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let rows = sqlx::query(
            "UPDATE daemon_pipelines SET schedule_cron = ?1, updated_at = ?2 WHERE id = ?3",
        )
        .bind(cron)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if rows == 0 {
            anyhow::bail!("pipeline '{id}' not found");
        }
        Ok(())
    }

    /// Delete a source by ID.
    pub async fn delete_source(&self, id: &str) -> Result<()> {
        let rows = sqlx::query("DELETE FROM daemon_sources WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if rows == 0 {
            anyhow::bail!("source '{id}' not found");
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Goals
    // -----------------------------------------------------------------------

    /// Create a new goal.
    pub async fn create_goal(&self, input: &CreateGoal) -> Result<DaemonGoal> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let metric_type = input.metric_type.to_string();
        let period = input.period.to_string();
        let unit = input.target_unit.as_deref().unwrap_or("USD");
        let priority = input.priority.unwrap_or(1);

        sqlx::query(
            "INSERT INTO daemon_goals (id, name, description, metric_type, target_value, target_unit, period, pipeline_id, current_value, status, priority, deadline, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0.0, 'active', ?9, ?10, ?11, ?11)",
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&metric_type)
        .bind(input.target_value)
        .bind(unit)
        .bind(&period)
        .bind(&input.pipeline_id)
        .bind(priority)
        .bind(input.deadline)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(DaemonGoal {
            id,
            name: input.name.clone(),
            description: input.description.clone(),
            metric_type,
            target_value: input.target_value,
            target_unit: unit.to_string(),
            period,
            pipeline_id: input.pipeline_id.clone(),
            current_value: 0.0,
            last_measured: None,
            status: "active".to_string(),
            priority,
            deadline: input.deadline,
            created_at: now,
            updated_at: now,
        })
    }

    /// List goals with optional filters.
    pub async fn list_goals(&self, filter: &GoalFilter) -> Result<Vec<DaemonGoal>> {
        let mut sql = String::from(
            "SELECT id, name, description, metric_type, target_value, target_unit, period, pipeline_id, current_value, last_measured, status, priority, deadline, created_at, updated_at FROM daemon_goals WHERE 1=1",
        );
        if let Some(status) = &filter.status {
            sql.push_str(&format!(" AND status = '{status}'"));
        }
        if let Some(pid) = &filter.pipeline_id {
            sql.push_str(&format!(" AND pipeline_id = '{pid}'"));
        }
        sql.push_str(" ORDER BY priority ASC, created_at DESC");
        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| DaemonGoal {
                id: r.get("id"),
                name: r.get("name"),
                description: r.get("description"),
                metric_type: r.get("metric_type"),
                target_value: r.get("target_value"),
                target_unit: r.get("target_unit"),
                period: r.get("period"),
                pipeline_id: r.get("pipeline_id"),
                current_value: r.get("current_value"),
                last_measured: r.get("last_measured"),
                status: r.get("status"),
                priority: r.get("priority"),
                deadline: r.get("deadline"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Update a goal's current measured value.
    pub async fn update_goal_current_value(&self, id: &str, value: f64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_goals SET current_value = ?1, last_measured = ?2, updated_at = ?2 WHERE id = ?3",
        )
        .bind(value)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update a goal's status.
    pub async fn set_goal_status(&self, id: &str, status: GoalStatus) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let status_str = status.to_string();
        sqlx::query("UPDATE daemon_goals SET status = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(&status_str)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Experiments (A/B testing)
    // -----------------------------------------------------------------------

    /// Create a new A/B experiment.
    pub async fn create_experiment(&self, input: &CreateExperiment) -> Result<DaemonExperiment> {
        let now = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        let exp_type = input.experiment_type.to_string();
        let min_days = input.min_duration_days.unwrap_or(7);

        sqlx::query(
            "INSERT INTO daemon_experiments \
             (id, content_id, pipeline_id, experiment_type, status, variant_a, variant_b, \
              active_variant, metric, metric_a, metric_b, winner, min_duration_days, \
              created_at, updated_at, completed_at) \
             VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?6, 'a', ?7, 0.0, 0.0, NULL, ?8, ?9, ?9, NULL)",
        )
        .bind(&id)
        .bind(&input.content_id)
        .bind(&input.pipeline_id)
        .bind(&exp_type)
        .bind(&input.variant_a)
        .bind(&input.variant_b)
        .bind(&input.metric)
        .bind(min_days)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_experiment(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("experiment not found after insert"))
    }

    /// Get an experiment by ID.
    pub async fn get_experiment(&self, id: &str) -> Result<Option<DaemonExperiment>> {
        let row =
            sqlx::query_as::<_, ExperimentRow>("SELECT * FROM daemon_experiments WHERE id = ?1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(DaemonExperiment::from))
    }

    /// List experiments with optional filters.
    pub async fn list_experiments(
        &self,
        filter: &ExperimentFilter,
    ) -> Result<Vec<DaemonExperiment>> {
        let mut sql = String::from("SELECT * FROM daemon_experiments WHERE 1=1");
        if filter.pipeline_id.is_some() {
            sql.push_str(" AND pipeline_id = ?1");
        }
        if filter.content_id.is_some() {
            sql.push_str(" AND content_id = ?2");
        }
        if filter.status.is_some() {
            sql.push_str(" AND status = ?3");
        }
        sql.push_str(" ORDER BY created_at DESC");
        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        let rows = sqlx::query_as::<_, ExperimentRow>(&sql)
            .bind(filter.pipeline_id.as_deref().unwrap_or(""))
            .bind(filter.content_id.as_deref().unwrap_or(""))
            .bind(filter.status.map(|s| s.to_string()).unwrap_or_default())
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(DaemonExperiment::from).collect())
    }

    /// Switch the active variant of an experiment.
    pub async fn switch_experiment_variant(&self, id: &str, variant: &str) -> Result<()> {
        anyhow::ensure!(
            variant == "a" || variant == "b",
            "variant must be 'a' or 'b'"
        );
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_experiments SET active_variant = ?1, updated_at = ?2 WHERE id = ?3",
        )
        .bind(variant)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update the metric values for an experiment.
    pub async fn update_experiment_metrics(
        &self,
        id: &str,
        metric_a: f64,
        metric_b: f64,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_experiments SET metric_a = ?1, metric_b = ?2, updated_at = ?3 WHERE id = ?4",
        )
        .bind(metric_a)
        .bind(metric_b)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Complete an experiment, recording the winner.
    pub async fn complete_experiment(&self, id: &str, winner: &str) -> Result<()> {
        anyhow::ensure!(winner == "a" || winner == "b", "winner must be 'a' or 'b'");
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_experiments SET status = 'completed', winner = ?1, \
             active_variant = ?1, completed_at = ?2, updated_at = ?2 WHERE id = ?3",
        )
        .bind(winner)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Cancel an experiment.
    pub async fn cancel_experiment(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE daemon_experiments SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List running experiments that are past their minimum duration.
    pub async fn list_mature_experiments(&self) -> Result<Vec<DaemonExperiment>> {
        let now = chrono::Utc::now().timestamp();
        let rows = sqlx::query_as::<_, ExperimentRow>(
            "SELECT * FROM daemon_experiments \
             WHERE status = 'running' \
             AND (created_at + min_duration_days * 86400) <= ?1 \
             ORDER BY created_at ASC",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(DaemonExperiment::from).collect())
    }

    // -----------------------------------------------------------------------
    // Prompt Scores
    // -----------------------------------------------------------------------

    /// Record a new prompt score entry (metrics start at zero).
    pub async fn create_prompt_score(&self, input: &CreatePromptScore) -> Result<PromptScore> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let params_str = serde_json::to_string(&input.params_json)?;

        sqlx::query(
            "INSERT INTO daemon_prompt_scores \
             (id, pipeline_id, content_id, prompt_hash, params_json, \
              avg_ctr, total_clicks, total_impressions, revenue_usd, composite_score, \
              created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0.0, 0, 0, 0.0, 0.0, ?6, ?6)",
        )
        .bind(&id)
        .bind(&input.pipeline_id)
        .bind(&input.content_id)
        .bind(&input.prompt_hash)
        .bind(&params_str)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(PromptScore {
            id,
            pipeline_id: input.pipeline_id.clone(),
            content_id: input.content_id.clone(),
            prompt_hash: input.prompt_hash.clone(),
            params_json: input.params_json.clone(),
            avg_ctr: 0.0,
            total_clicks: 0,
            total_impressions: 0,
            revenue_usd: 0.0,
            composite_score: 0.0,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update metrics for a prompt score record.
    pub async fn update_prompt_score_metrics(
        &self,
        id: &str,
        ctr: f64,
        clicks: i64,
        impressions: i64,
        revenue: f64,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        // Composite: weighted CTR (60%) + normalized revenue (40%).
        let composite = compute_composite_score(ctr, clicks, revenue);
        sqlx::query(
            "UPDATE daemon_prompt_scores \
             SET avg_ctr = ?1, total_clicks = ?2, total_impressions = ?3, \
                 revenue_usd = ?4, composite_score = ?5, updated_at = ?6 \
             WHERE id = ?7",
        )
        .bind(ctr)
        .bind(clicks)
        .bind(impressions)
        .bind(revenue)
        .bind(composite)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get prompt scores for a pipeline, ordered by composite score descending.
    pub async fn list_prompt_scores(&self, pipeline_id: &str) -> Result<Vec<PromptScore>> {
        let rows = sqlx::query_as::<_, PromptScoreRow>(
            "SELECT * FROM daemon_prompt_scores WHERE pipeline_id = ?1 \
             ORDER BY composite_score DESC",
        )
        .bind(pipeline_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(PromptScore::from).collect())
    }

    /// Get aggregated performance per prompt_hash for a pipeline.
    pub async fn prompt_performance_summary(
        &self,
        pipeline_id: &str,
    ) -> Result<Vec<PromptPerformanceSummary>> {
        let rows = sqlx::query_as::<_, PromptSummaryRow>(
            "SELECT prompt_hash, params_json, \
                    COUNT(*) as content_count, \
                    AVG(avg_ctr) as avg_ctr, \
                    AVG(total_clicks) as avg_clicks, \
                    SUM(revenue_usd) as total_revenue, \
                    AVG(composite_score) as composite_score \
             FROM daemon_prompt_scores \
             WHERE pipeline_id = ?1 \
             GROUP BY prompt_hash \
             ORDER BY composite_score DESC",
        )
        .bind(pipeline_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(PromptPerformanceSummary::from)
            .collect())
    }

    /// Get the score for a specific content_id (if tracked).
    pub async fn get_prompt_score_by_content(
        &self,
        content_id: &str,
    ) -> Result<Option<PromptScore>> {
        let row = sqlx::query_as::<_, PromptScoreRow>(
            "SELECT * FROM daemon_prompt_scores WHERE content_id = ?1",
        )
        .bind(content_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(PromptScore::from))
    }

    /// Get the best-performing prompt hash for a pipeline.
    pub async fn best_prompt_hash(&self, pipeline_id: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT prompt_hash FROM daemon_prompt_scores \
             WHERE pipeline_id = ?1 \
             GROUP BY prompt_hash \
             HAVING COUNT(*) >= 3 \
             ORDER BY AVG(composite_score) DESC \
             LIMIT 1",
        )
        .bind(pipeline_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }
}

/// Compute a composite score from CTR, clicks, and revenue.
/// Weights: CTR 40%, clicks 30% (log-normalized), revenue 30%.
fn compute_composite_score(ctr: f64, clicks: i64, revenue: f64) -> f64 {
    let ctr_component = ctr * 100.0; // e.g. 3.5% -> 3.5
    let clicks_component = (clicks as f64 + 1.0).ln() * 10.0; // log-normalize
    let revenue_component = revenue * 10.0; // scale up small values
    ctr_component * 0.4 + clicks_component * 0.3 + revenue_component * 0.3
}

// ---------------------------------------------------------------------------
// Helper: content hashing for deduplication
// ---------------------------------------------------------------------------

fn compute_content_hash(body: &str) -> String {
    use sha2::Digest;
    use sha2::Sha256;
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Internal row types (sqlx::FromRow) — map directly to SQL columns
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct PipelineRow {
    id: String,
    name: String,
    strategy: String,
    config_json: String,
    schedule_cron: String,
    enabled: bool,
    max_retries: i32,
    retry_delay_sec: i32,
    created_at: i64,
    updated_at: i64,
}

impl From<PipelineRow> for DaemonPipeline {
    fn from(r: PipelineRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            strategy: r.strategy,
            config_json: r.config_json,
            schedule_cron: r.schedule_cron,
            enabled: r.enabled,
            max_retries: r.max_retries,
            retry_delay_sec: r.retry_delay_sec,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct JobRow {
    id: String,
    pipeline_id: String,
    status: String,
    attempt: i32,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    input_json: Option<String>,
    output_json: Option<String>,
    error_message: Option<String>,
    error_stack: Option<String>,
    duration_ms: Option<i64>,
    created_at: i64,
}

impl From<JobRow> for DaemonJob {
    fn from(r: JobRow) -> Self {
        Self {
            id: r.id,
            pipeline_id: r.pipeline_id,
            status: r.status,
            attempt: r.attempt,
            started_at: r.started_at,
            completed_at: r.completed_at,
            input_json: r.input_json,
            output_json: r.output_json,
            error_message: r.error_message,
            error_stack: r.error_stack,
            duration_ms: r.duration_ms,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ContentRow {
    id: String,
    job_id: String,
    pipeline_id: String,
    content_type: String,
    platform: String,
    title: Option<String>,
    slug: Option<String>,
    url: Option<String>,
    status: String,
    word_count: Option<i32>,
    llm_model: Option<String>,
    llm_tokens_used: Option<i64>,
    llm_cost_usd: Option<f64>,
    content_hash: Option<String>,
    created_at: i64,
    published_at: Option<i64>,
}

impl From<ContentRow> for DaemonContent {
    fn from(r: ContentRow) -> Self {
        Self {
            id: r.id,
            job_id: r.job_id,
            pipeline_id: r.pipeline_id,
            content_type: r.content_type,
            platform: r.platform,
            title: r.title,
            slug: r.slug,
            url: r.url,
            status: r.status,
            word_count: r.word_count,
            llm_model: r.llm_model,
            llm_tokens_used: r.llm_tokens_used,
            llm_cost_usd: r.llm_cost_usd,
            content_hash: r.content_hash,
            created_at: r.created_at,
            published_at: r.published_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SourceRow {
    id: String,
    pipeline_id: String,
    source_type: String,
    name: String,
    url: String,
    scrape_selector: Option<String>,
    last_checked_at: Option<i64>,
    last_content_hash: Option<String>,
    check_interval_sec: i32,
    enabled: bool,
    created_at: i64,
    updated_at: i64,
}

impl From<SourceRow> for DaemonSource {
    fn from(r: SourceRow) -> Self {
        Self {
            id: r.id,
            pipeline_id: r.pipeline_id,
            source_type: r.source_type,
            name: r.name,
            url: r.url,
            scrape_selector: r.scrape_selector,
            last_checked_at: r.last_checked_at,
            last_content_hash: r.last_content_hash,
            check_interval_sec: r.check_interval_sec,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct LogRow {
    id: i64,
    job_id: Option<String>,
    pipeline_id: String,
    level: String,
    message: String,
    context_json: Option<String>,
    created_at: i64,
}

impl From<LogRow> for DaemonLog {
    fn from(r: LogRow) -> Self {
        Self {
            id: r.id,
            job_id: r.job_id,
            pipeline_id: r.pipeline_id,
            level: r.level,
            message: r.message,
            context_json: r.context_json,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProposalRow {
    id: String,
    pipeline_id: Option<String>,
    action_type: String,
    title: String,
    description: String,
    reasoning: String,
    confidence: f64,
    risk_level: String,
    status: String,
    proposed_config: Option<String>,
    metrics_snapshot: Option<String>,
    auto_approvable: bool,
    created_at: i64,
    reviewed_at: Option<i64>,
    executed_at: Option<i64>,
    expires_at: Option<i64>,
}

impl From<ProposalRow> for DaemonProposal {
    fn from(r: ProposalRow) -> Self {
        Self {
            id: r.id,
            pipeline_id: r.pipeline_id,
            action_type: r.action_type,
            title: r.title,
            description: r.description,
            reasoning: r.reasoning,
            confidence: r.confidence,
            risk_level: r.risk_level,
            status: r.status,
            proposed_config: r.proposed_config,
            metrics_snapshot: r.metrics_snapshot,
            auto_approvable: r.auto_approvable,
            created_at: r.created_at,
            reviewed_at: r.reviewed_at,
            executed_at: r.executed_at,
            expires_at: r.expires_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct RevenueRow {
    id: String,
    content_id: Option<String>,
    pipeline_id: String,
    source: String,
    amount: f64,
    currency: String,
    period_start: i64,
    period_end: i64,
    external_id: Option<String>,
    metadata_json: Option<String>,
    created_at: i64,
}

impl From<RevenueRow> for DaemonRevenue {
    fn from(r: RevenueRow) -> Self {
        Self {
            id: r.id,
            content_id: r.content_id,
            pipeline_id: r.pipeline_id,
            source: r.source,
            amount: r.amount,
            currency: r.currency,
            period_start: r.period_start,
            period_end: r.period_end,
            external_id: r.external_id,
            metadata_json: r.metadata_json,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ExperimentRow {
    id: String,
    content_id: String,
    pipeline_id: String,
    experiment_type: String,
    status: String,
    variant_a: String,
    variant_b: String,
    active_variant: String,
    metric: String,
    metric_a: f64,
    metric_b: f64,
    winner: Option<String>,
    min_duration_days: i32,
    created_at: i64,
    updated_at: i64,
    completed_at: Option<i64>,
}

impl From<ExperimentRow> for DaemonExperiment {
    fn from(r: ExperimentRow) -> Self {
        Self {
            id: r.id,
            content_id: r.content_id,
            pipeline_id: r.pipeline_id,
            experiment_type: r.experiment_type,
            status: r.status,
            variant_a: r.variant_a,
            variant_b: r.variant_b,
            active_variant: r.active_variant,
            metric: r.metric,
            metric_a: r.metric_a,
            metric_b: r.metric_b,
            winner: r.winner,
            min_duration_days: r.min_duration_days,
            created_at: r.created_at,
            updated_at: r.updated_at,
            completed_at: r.completed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PromptScoreRow {
    id: String,
    pipeline_id: String,
    content_id: String,
    prompt_hash: String,
    params_json: String,
    avg_ctr: f64,
    total_clicks: i64,
    total_impressions: i64,
    revenue_usd: f64,
    composite_score: f64,
    created_at: i64,
    updated_at: i64,
}

impl From<PromptScoreRow> for PromptScore {
    fn from(r: PromptScoreRow) -> Self {
        Self {
            id: r.id,
            pipeline_id: r.pipeline_id,
            content_id: r.content_id,
            prompt_hash: r.prompt_hash,
            params_json: serde_json::from_str(&r.params_json).unwrap_or_default(),
            avg_ctr: r.avg_ctr,
            total_clicks: r.total_clicks,
            total_impressions: r.total_impressions,
            revenue_usd: r.revenue_usd,
            composite_score: r.composite_score,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PromptSummaryRow {
    prompt_hash: String,
    params_json: String,
    content_count: i64,
    avg_ctr: f64,
    avg_clicks: f64,
    total_revenue: f64,
    composite_score: f64,
}

impl From<PromptSummaryRow> for PromptPerformanceSummary {
    fn from(r: PromptSummaryRow) -> Self {
        Self {
            prompt_hash: r.prompt_hash,
            params_json: serde_json::from_str(&r.params_json).unwrap_or_default(),
            content_count: r.content_count,
            avg_ctr: r.avg_ctr,
            avg_clicks: r.avg_clicks,
            total_revenue: r.total_revenue,
            composite_score: r.composite_score,
        }
    }
}

// ---------------------------------------------------------------------------
// Migration SQL
// ---------------------------------------------------------------------------

const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS daemon_pipelines (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    strategy        TEXT NOT NULL CHECK(strategy IN ('seo_blog', 'youtube_shorts', 'saas_api', 'metrics_collector', 'strategy_analyzer', 'ab_tester', 'prompt_optimizer')),
    config_json     TEXT NOT NULL DEFAULT '{}',
    schedule_cron   TEXT NOT NULL DEFAULT '0 3 * * *',
    enabled         INTEGER NOT NULL DEFAULT 1,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    retry_delay_sec INTEGER NOT NULL DEFAULT 300,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS daemon_jobs (
    id              TEXT PRIMARY KEY,
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    attempt         INTEGER NOT NULL DEFAULT 1,
    started_at      INTEGER,
    completed_at    INTEGER,
    input_json      TEXT,
    output_json     TEXT,
    error_message   TEXT,
    error_stack     TEXT,
    duration_ms     INTEGER,
    created_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS daemon_content (
    id              TEXT PRIMARY KEY,
    job_id          TEXT NOT NULL REFERENCES daemon_jobs(id),
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    content_type    TEXT NOT NULL CHECK(content_type IN ('article', 'video_short', 'api_response', 'pdf', 'image')),
    platform        TEXT NOT NULL CHECK(platform IN ('wordpress', 'ghost', 'youtube', 'tiktok', 'gumroad', 'stripe', 'local')),
    title           TEXT,
    slug            TEXT,
    url             TEXT,
    status          TEXT NOT NULL DEFAULT 'draft'
                    CHECK(status IN ('draft', 'rendering', 'uploading', 'published', 'failed', 'archived')),
    word_count      INTEGER,
    llm_model       TEXT,
    llm_tokens_used INTEGER,
    llm_cost_usd    REAL,
    content_hash    TEXT,
    created_at      INTEGER NOT NULL,
    published_at    INTEGER
);

CREATE TABLE IF NOT EXISTS daemon_sources (
    id              TEXT PRIMARY KEY,
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    source_type     TEXT NOT NULL CHECK(source_type IN ('rss', 'webpage', 'api', 'pdf_url', 'youtube_channel')),
    name            TEXT NOT NULL,
    url             TEXT NOT NULL,
    scrape_selector TEXT,
    last_checked_at INTEGER,
    last_content_hash TEXT,
    check_interval_sec INTEGER NOT NULL DEFAULT 86400,
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS daemon_metrics (
    id              TEXT PRIMARY KEY,
    content_id      TEXT REFERENCES daemon_content(id),
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    metric_type     TEXT NOT NULL CHECK(metric_type IN ('views', 'clicks', 'revenue', 'impressions', 'subscribers', 'ctr')),
    value           REAL NOT NULL,
    currency        TEXT DEFAULT 'USD',
    period_start    INTEGER NOT NULL,
    period_end      INTEGER NOT NULL,
    source          TEXT NOT NULL,
    created_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS daemon_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id          TEXT,
    pipeline_id     TEXT NOT NULL,
    level           TEXT NOT NULL CHECK(level IN ('trace', 'debug', 'info', 'warn', 'error')),
    message         TEXT NOT NULL,
    context_json    TEXT,
    created_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_pipeline_status ON daemon_jobs(pipeline_id, status);
CREATE INDEX IF NOT EXISTS idx_jobs_created ON daemon_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_content_pipeline ON daemon_content(pipeline_id, status);
CREATE INDEX IF NOT EXISTS idx_content_published ON daemon_content(published_at DESC);
CREATE INDEX IF NOT EXISTS idx_sources_pipeline ON daemon_sources(pipeline_id, enabled);
CREATE INDEX IF NOT EXISTS idx_sources_next_check ON daemon_sources(last_checked_at);
CREATE INDEX IF NOT EXISTS idx_metrics_pipeline_type ON daemon_metrics(pipeline_id, metric_type, period_start);
CREATE INDEX IF NOT EXISTS idx_logs_job ON daemon_logs(job_id, created_at);
CREATE INDEX IF NOT EXISTS idx_logs_pipeline ON daemon_logs(pipeline_id, created_at DESC);

CREATE TABLE IF NOT EXISTS daemon_proposals (
    id              TEXT PRIMARY KEY,
    pipeline_id     TEXT REFERENCES daemon_pipelines(id),
    action_type     TEXT NOT NULL CHECK(action_type IN (
        'create_pipeline', 'modify_pipeline', 'disable_pipeline',
        'change_niche', 'change_frequency', 'add_source', 'remove_source',
        'scale_up', 'scale_down', 'change_model', 'custom'
    )),
    title           TEXT NOT NULL,
    description     TEXT NOT NULL,
    reasoning       TEXT NOT NULL,
    confidence      REAL NOT NULL,
    risk_level      TEXT NOT NULL CHECK(risk_level IN ('low', 'medium', 'high')),
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending', 'approved', 'rejected', 'expired', 'executed', 'failed')),
    proposed_config TEXT,
    metrics_snapshot TEXT,
    auto_approvable INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    reviewed_at     INTEGER,
    executed_at     INTEGER,
    expires_at      INTEGER
);

CREATE TABLE IF NOT EXISTS daemon_revenue (
    id              TEXT PRIMARY KEY,
    content_id      TEXT REFERENCES daemon_content(id),
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    source          TEXT NOT NULL CHECK(source IN (
        'adsense', 'affiliate', 'gumroad', 'stripe', 'manual', 'estimated'
    )),
    amount          REAL NOT NULL,
    currency        TEXT NOT NULL DEFAULT 'USD',
    period_start    INTEGER NOT NULL,
    period_end      INTEGER NOT NULL,
    external_id     TEXT,
    metadata_json   TEXT,
    created_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_proposals_status ON daemon_proposals(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_proposals_pipeline ON daemon_proposals(pipeline_id, status);
CREATE INDEX IF NOT EXISTS idx_revenue_pipeline ON daemon_revenue(pipeline_id, period_start);
CREATE INDEX IF NOT EXISTS idx_revenue_period ON daemon_revenue(period_start, period_end);
CREATE INDEX IF NOT EXISTS idx_revenue_source ON daemon_revenue(source, period_start);

CREATE TABLE IF NOT EXISTS daemon_goals (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT,
    metric_type     TEXT NOT NULL CHECK(metric_type IN (
        'revenue', 'content_count', 'pageviews', 'clicks',
        'ctr', 'subscribers', 'cost_limit', 'custom'
    )),
    target_value    REAL NOT NULL,
    target_unit     TEXT NOT NULL DEFAULT 'USD',
    period          TEXT NOT NULL DEFAULT 'monthly' CHECK(period IN (
        'daily', 'weekly', 'monthly', 'quarterly', 'yearly'
    )),
    pipeline_id     TEXT,
    current_value   REAL NOT NULL DEFAULT 0.0,
    last_measured   INTEGER,
    status          TEXT NOT NULL DEFAULT 'active' CHECK(status IN (
        'active', 'achieved', 'paused', 'failed', 'archived'
    )),
    priority        INTEGER NOT NULL DEFAULT 1,
    deadline        INTEGER,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_goals_status ON daemon_goals(status);
CREATE INDEX IF NOT EXISTS idx_goals_pipeline ON daemon_goals(pipeline_id);

CREATE TABLE IF NOT EXISTS daemon_experiments (
    id                TEXT PRIMARY KEY,
    content_id        TEXT NOT NULL,
    pipeline_id       TEXT NOT NULL,
    experiment_type   TEXT NOT NULL CHECK(experiment_type IN ('title', 'meta_description', 'headline', 'custom')),
    status            TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'cancelled')),
    variant_a         TEXT NOT NULL,
    variant_b         TEXT NOT NULL,
    active_variant    TEXT NOT NULL DEFAULT 'a' CHECK(active_variant IN ('a', 'b')),
    metric            TEXT NOT NULL DEFAULT 'ctr',
    metric_a          REAL NOT NULL DEFAULT 0.0,
    metric_b          REAL NOT NULL DEFAULT 0.0,
    winner            TEXT CHECK(winner IN ('a', 'b') OR winner IS NULL),
    min_duration_days INTEGER NOT NULL DEFAULT 7,
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL,
    completed_at      INTEGER
);

CREATE INDEX IF NOT EXISTS idx_experiments_status ON daemon_experiments(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_experiments_content ON daemon_experiments(content_id);
CREATE INDEX IF NOT EXISTS idx_experiments_pipeline ON daemon_experiments(pipeline_id);

CREATE TABLE IF NOT EXISTS daemon_prompt_scores (
    id                TEXT PRIMARY KEY,
    pipeline_id       TEXT NOT NULL REFERENCES daemon_pipelines(id),
    content_id        TEXT NOT NULL REFERENCES daemon_content(id),
    prompt_hash       TEXT NOT NULL,
    params_json       TEXT NOT NULL DEFAULT '{}',
    avg_ctr           REAL NOT NULL DEFAULT 0.0,
    total_clicks      INTEGER NOT NULL DEFAULT 0,
    total_impressions INTEGER NOT NULL DEFAULT 0,
    revenue_usd       REAL NOT NULL DEFAULT 0.0,
    composite_score   REAL NOT NULL DEFAULT 0.0,
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_prompt_scores_hash ON daemon_prompt_scores(prompt_hash);
CREATE INDEX IF NOT EXISTS idx_prompt_scores_pipeline ON daemon_prompt_scores(pipeline_id, composite_score DESC);
CREATE INDEX IF NOT EXISTS idx_prompt_scores_content ON daemon_prompt_scores(content_id);
"#;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_pipeline_crud() {
        let db = DaemonDb::open_memory().await.expect("open db");

        let input = CreatePipeline {
            id: "test-seo".to_string(),
            name: "Test SEO Pipeline".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({"niche": "tech"}),
            schedule_cron: "0 3 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };

        let pipeline = db.create_pipeline(&input).await.expect("create pipeline");
        assert_eq!(pipeline.id, "test-seo");
        assert_eq!(pipeline.strategy, "seo_blog");
        assert!(pipeline.enabled);

        // List
        let all = db.list_pipelines(false).await.expect("list");
        assert_eq!(all.len(), 1);

        // Disable
        db.set_pipeline_enabled("test-seo", false)
            .await
            .expect("disable");
        let enabled = db.list_pipelines(true).await.expect("list enabled");
        assert!(enabled.is_empty());
    }

    #[tokio::test]
    async fn test_job_lifecycle() {
        let db = DaemonDb::open_memory().await.expect("open db");

        // Create pipeline first
        let input = CreatePipeline {
            id: "p1".to_string(),
            name: "Pipeline 1".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&input).await.expect("create pipeline");

        // Create + start + complete job
        let job = db.create_job("p1").await.expect("create job");
        assert_eq!(job.status, "pending");

        db.start_job(&job.id).await.expect("start job");
        db.complete_job(&job.id, Some(r#"{"articles": 3}"#))
            .await
            .expect("complete job");

        let filter = JobFilter {
            pipeline_id: Some("p1".to_string()),
            ..Default::default()
        };
        let jobs = db.list_jobs(&filter).await.expect("list jobs");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, "completed");

        // Running count should be 0
        let running = db.count_running_jobs().await.expect("count");
        assert_eq!(running, 0);
    }

    #[tokio::test]
    async fn test_content_dedup() {
        let db = DaemonDb::open_memory().await.expect("open db");

        let input = CreatePipeline {
            id: "p1".to_string(),
            name: "P1".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&input).await.expect("create pipeline");
        let job = db.create_job("p1").await.expect("create job");

        let output = ContentOutput {
            content_type: ContentType::Article,
            platform: Platform::Wordpress,
            title: "Test Article".to_string(),
            slug: "test-article".to_string(),
            body: "This is the article body for testing.".to_string(),
            url: None,
            word_count: Some(7),
            llm_model: "gemini-2.0-flash".to_string(),
            llm_tokens_used: 150,
            llm_cost_usd: Some(0.0),
        };

        let content = db
            .create_content(&job.id, "p1", &output)
            .await
            .expect("create content");
        assert_eq!(content.status, "draft");

        // Same body should detect duplicate
        let hash = compute_content_hash(&output.body);
        let exists = db.content_exists_by_hash(&hash).await.expect("check hash");
        assert!(exists);

        // Different body should not
        let exists = db
            .content_exists_by_hash("different_hash")
            .await
            .expect("check");
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_sources_due_for_check() {
        let db = DaemonDb::open_memory().await.expect("open db");

        let input = CreatePipeline {
            id: "p1".to_string(),
            name: "P1".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&input).await.expect("create pipeline");

        let source_input = CreateSource {
            pipeline_id: "p1".to_string(),
            source_type: SourceType::Rss,
            name: "Test RSS".to_string(),
            url: "https://example.com/feed.xml".to_string(),
            scrape_selector: None,
            check_interval_sec: Some(3600),
        };
        db.create_source(&source_input)
            .await
            .expect("create source");

        // Never checked — should be due
        let due = db.get_sources_due_for_check("p1").await.expect("get due");
        assert_eq!(due.len(), 1);

        // After checking — should not be due (just checked)
        db.update_source_checked(&due[0].id, "hash123")
            .await
            .expect("update");
        let due = db.get_sources_due_for_check("p1").await.expect("get due");
        assert!(due.is_empty());
    }

    #[tokio::test]
    async fn test_logs() {
        let db = DaemonDb::open_memory().await.expect("open db");

        db.insert_log("p1", None, LogLevel::Info, "Pipeline started", None)
            .await
            .expect("insert log");
        db.insert_log(
            "p1",
            Some("job-1"),
            LogLevel::Error,
            "Failed to fetch RSS",
            Some(&serde_json::json!({"url": "https://example.com"})),
        )
        .await
        .expect("insert log");

        let filter = LogFilter {
            pipeline_id: Some("p1".to_string()),
            ..Default::default()
        };
        let logs = db.list_logs(&filter).await.expect("list logs");
        assert_eq!(logs.len(), 2);

        // Verify both log levels are present (order depends on autoincrement id DESC).
        let levels: Vec<&str> = logs.iter().map(|l| l.level.as_str()).collect();
        assert!(levels.contains(&"info"));
        assert!(levels.contains(&"error"));
    }

    #[tokio::test]
    async fn test_proposal_lifecycle() {
        let db = DaemonDb::open_memory().await.expect("open db");

        // Create a pipeline for the proposal to reference.
        let pipe_input = CreatePipeline {
            id: "p1".to_string(),
            name: "SEO Pipeline".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 3 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&pipe_input)
            .await
            .expect("create pipeline");

        // Create a proposal.
        let proposal_input = CreateProposal {
            pipeline_id: Some("p1".to_string()),
            action_type: ActionType::ScaleUp,
            title: "Increase publishing frequency".to_string(),
            description: "CTR is above 5%, scale from 3 to 5 articles/day".to_string(),
            reasoning: "7-day CTR average is 5.2%, well above the 3% threshold. Revenue per click is $0.12, suggesting higher volume would be profitable.".to_string(),
            confidence: 0.87,
            risk_level: RiskLevel::Low,
            proposed_config: Some(serde_json::json!({"articles_per_day": 5})),
            metrics_snapshot: Some(serde_json::json!({"ctr": 0.052, "revenue_7d": 3.45})),
            auto_approvable: true,
            expires_in_hours: Some(72),
        };
        let proposal = db
            .create_proposal(&proposal_input)
            .await
            .expect("create proposal");
        assert_eq!(proposal.status, "pending");
        assert_eq!(proposal.action_type, "scale_up");
        assert_eq!(proposal.risk_level, "low");
        assert!(proposal.auto_approvable);
        assert!(proposal.expires_at.is_some());

        // List pending proposals.
        let filter = ProposalFilter {
            status: Some(ProposalStatus::Pending),
            ..Default::default()
        };
        let pending = db.list_proposals(&filter).await.expect("list proposals");
        assert_eq!(pending.len(), 1);

        // Count pending.
        let count = db.count_pending_proposals().await.expect("count");
        assert_eq!(count, 1);

        // Get by ID.
        let fetched = db
            .get_proposal(&proposal.id)
            .await
            .expect("get proposal")
            .expect("proposal should exist");
        assert_eq!(fetched.id, proposal.id);
        assert_eq!(fetched.title, "Increase publishing frequency");

        // Approve.
        db.approve_proposal(&proposal.id)
            .await
            .expect("approve proposal");
        let approved = db
            .get_proposal(&proposal.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(approved.status, "approved");
        assert!(approved.reviewed_at.is_some());

        // Mark executed.
        db.mark_proposal_executed(&proposal.id)
            .await
            .expect("mark executed");
        let executed = db
            .get_proposal(&proposal.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(executed.status, "executed");
        assert!(executed.executed_at.is_some());

        // Count pending should now be 0.
        let count = db.count_pending_proposals().await.expect("count");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_proposal_reject() {
        let db = DaemonDb::open_memory().await.expect("open db");

        let proposal_input = CreateProposal {
            pipeline_id: None,
            action_type: ActionType::CreatePipeline,
            title: "Create new niche pipeline".to_string(),
            description: "Suggest creating a pipeline for 'receitas' niche".to_string(),
            reasoning: "Trending topic with low competition.".to_string(),
            confidence: 0.65,
            risk_level: RiskLevel::Medium,
            proposed_config: None,
            metrics_snapshot: None,
            auto_approvable: false,
            expires_in_hours: None,
        };
        let proposal = db
            .create_proposal(&proposal_input)
            .await
            .expect("create proposal");

        db.reject_proposal(&proposal.id)
            .await
            .expect("reject proposal");
        let rejected = db
            .get_proposal(&proposal.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(rejected.status, "rejected");
        assert!(rejected.reviewed_at.is_some());

        // Attempting to approve a rejected proposal should fail.
        let result = db.approve_proposal(&proposal.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revenue_crud() {
        let db = DaemonDb::open_memory().await.expect("open db");

        // Create a pipeline.
        let pipe_input = CreatePipeline {
            id: "p1".to_string(),
            name: "SEO Blog".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 3 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&pipe_input)
            .await
            .expect("create pipeline");

        let now = chrono::Utc::now().timestamp();
        let day_ago = now - 86400;

        // Record some revenue.
        let rev1 = CreateRevenue {
            content_id: None,
            pipeline_id: "p1".to_string(),
            source: RevenueSource::Estimated,
            amount: 1.50,
            currency: None,
            period_start: day_ago,
            period_end: now,
            external_id: None,
            metadata: Some(serde_json::json!({"clicks": 30, "cpc": 0.05})),
        };
        let revenue = db.create_revenue(&rev1).await.expect("create revenue");
        assert_eq!(revenue.amount, 1.50);
        assert_eq!(revenue.currency, "USD");
        assert_eq!(revenue.source, "estimated");

        let rev2 = CreateRevenue {
            content_id: None,
            pipeline_id: "p1".to_string(),
            source: RevenueSource::Adsense,
            amount: 3.25,
            currency: Some("USD".to_string()),
            period_start: day_ago,
            period_end: now,
            external_id: Some("txn_123".to_string()),
            metadata: None,
        };
        db.create_revenue(&rev2).await.expect("create revenue 2");

        // List revenue.
        let filter = RevenueFilter {
            pipeline_id: Some("p1".to_string()),
            ..Default::default()
        };
        let records = db.list_revenue(&filter).await.expect("list revenue");
        assert_eq!(records.len(), 2);

        // Revenue summary.
        let summary = db.revenue_summary(7).await.expect("summary");
        assert!((summary.total_usd - 4.75).abs() < 0.01);
        assert_eq!(summary.period_days, 7);
        assert_eq!(summary.by_pipeline.len(), 1);
        assert_eq!(summary.by_pipeline[0].pipeline_id, "p1");
        assert_eq!(summary.by_source.len(), 2);
    }

    #[tokio::test]
    async fn test_goal_lifecycle() {
        let db = DaemonDb::open_memory().await.expect("open db");

        // Create a goal.
        let input = CreateGoal {
            name: "Monthly Revenue $50".to_string(),
            description: Some("First revenue target".to_string()),
            metric_type: GoalMetricType::Revenue,
            target_value: 50.0,
            target_unit: Some("USD".to_string()),
            period: GoalPeriod::Monthly,
            pipeline_id: None,
            priority: Some(1),
            deadline: None,
        };

        let goal = db.create_goal(&input).await.expect("create goal");
        assert_eq!(goal.name, "Monthly Revenue $50");
        assert_eq!(goal.metric_type, "revenue");
        assert_eq!(goal.target_value, 50.0);
        assert_eq!(goal.current_value, 0.0);
        assert_eq!(goal.status, "active");
        assert_eq!(goal.priority, 1);

        // List active goals.
        let active = db
            .list_goals(&GoalFilter {
                status: Some(GoalStatus::Active),
                ..Default::default()
            })
            .await
            .expect("list goals");
        assert_eq!(active.len(), 1);

        // Update current value.
        db.update_goal_current_value(&goal.id, 25.0)
            .await
            .expect("update value");
        let updated = db
            .list_goals(&GoalFilter::default())
            .await
            .expect("list")
            .into_iter()
            .find(|g| g.id == goal.id)
            .expect("find goal");
        assert!((updated.current_value - 25.0).abs() < 0.01);
        assert!(updated.last_measured.is_some());

        // Pause.
        db.set_goal_status(&goal.id, GoalStatus::Paused)
            .await
            .expect("pause");
        let paused = db
            .list_goals(&GoalFilter {
                status: Some(GoalStatus::Active),
                ..Default::default()
            })
            .await
            .expect("list active");
        assert!(paused.is_empty());

        // Resume.
        db.set_goal_status(&goal.id, GoalStatus::Active)
            .await
            .expect("resume");
        let active = db
            .list_goals(&GoalFilter {
                status: Some(GoalStatus::Active),
                ..Default::default()
            })
            .await
            .expect("list active");
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_config_update() {
        let db = DaemonDb::open_memory().await.expect("open db");

        let input = CreatePipeline {
            id: "p1".to_string(),
            name: "Pipeline 1".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({"niche": "tech"}),
            schedule_cron: "0 3 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&input).await.expect("create pipeline");

        // Update config.
        let new_config = serde_json::json!({"niche": "finance", "articles_per_day": 5});
        db.update_pipeline_config("p1", &new_config)
            .await
            .expect("update config");

        let pipeline = db.get_pipeline("p1").await.expect("get").expect("exists");
        let config: serde_json::Value = serde_json::from_str(&pipeline.config_json).expect("parse");
        assert_eq!(config["niche"], "finance");
        assert_eq!(config["articles_per_day"], 5);

        // Update schedule.
        db.update_pipeline_schedule("p1", "0 */6 * * *")
            .await
            .expect("update schedule");
        let pipeline = db.get_pipeline("p1").await.expect("get").expect("exists");
        assert_eq!(pipeline.schedule_cron, "0 */6 * * *");
    }

    #[tokio::test]
    async fn test_create_job_with_attempt() {
        let db = DaemonDb::open_memory().await.expect("open db");

        let input = CreatePipeline {
            id: "p1".to_string(),
            name: "P1".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&input).await.expect("create pipeline");

        // Normal job has attempt 1.
        let job1 = db.create_job("p1").await.expect("create job");
        assert_eq!(job1.attempt, 1);

        // Retry job has attempt 2.
        let job2 = db
            .create_job_with_attempt("p1", 2)
            .await
            .expect("create retry job");
        assert_eq!(job2.attempt, 2);
        assert_eq!(job2.pipeline_id, "p1");
        assert_eq!(job2.status, "pending");

        // Third retry.
        let job3 = db
            .create_job_with_attempt("p1", 3)
            .await
            .expect("create retry job 3");
        assert_eq!(job3.attempt, 3);

        // All three jobs exist.
        let filter = JobFilter {
            pipeline_id: Some("p1".to_string()),
            ..Default::default()
        };
        let jobs = db.list_jobs(&filter).await.expect("list jobs");
        assert_eq!(jobs.len(), 3);
    }

    #[tokio::test]
    async fn test_experiment_lifecycle() {
        let db = DaemonDb::open_memory().await.expect("open db");

        let pipe = CreatePipeline {
            id: "p1".to_string(),
            name: "P1".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: serde_json::json!({}),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        };
        db.create_pipeline(&pipe).await.expect("create pipeline");
        let job = db.create_job("p1").await.expect("create job");
        let content = db
            .create_content(
                &job.id,
                "p1",
                &ContentOutput {
                    content_type: ContentType::Article,
                    platform: Platform::Wordpress,
                    title: "Test Article".to_string(),
                    slug: "test-article".to_string(),
                    body: "Test body for experiment lifecycle".to_string(),
                    url: None,
                    word_count: Some(5),
                    llm_model: "test".to_string(),
                    llm_tokens_used: 10,
                    llm_cost_usd: None,
                },
            )
            .await
            .expect("create content");

        // Create experiment.
        let exp = db
            .create_experiment(&CreateExperiment {
                content_id: content.id.clone(),
                pipeline_id: "p1".to_string(),
                experiment_type: ExperimentType::Title,
                variant_a: "Original".to_string(),
                variant_b: "Better Title".to_string(),
                metric: "ctr".to_string(),
                min_duration_days: Some(7),
            })
            .await
            .expect("create experiment");

        assert_eq!(exp.status, "running");
        assert_eq!(exp.active_variant, "a");
        assert!(exp.winner.is_none());
        assert_eq!(exp.min_duration_days, 7);

        // Update metrics.
        db.update_experiment_metrics(&exp.id, 3.2, 4.1)
            .await
            .expect("update metrics");

        // Complete.
        db.complete_experiment(&exp.id, "b")
            .await
            .expect("complete");

        let completed = db
            .get_experiment(&exp.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(completed.status, "completed");
        assert_eq!(completed.winner.as_deref(), Some("b"));
        assert!((completed.metric_a - 3.2).abs() < 0.01);
        assert!((completed.metric_b - 4.1).abs() < 0.01);
    }
}
