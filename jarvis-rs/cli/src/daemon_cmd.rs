//! CLI commands for managing the Jarvis daemon automation system.
//!
//! These commands allow inspecting and controlling daemon pipelines, jobs,
//! content, and logs from the interactive CLI — without needing the daemon
//! to be running.

use anyhow::Result;
use chrono::{TimeZone, Utc};
use clap::{Args, Subcommand};
use jarvis_daemon_common::{
    ContentFilter, ContentStatus, CreateGoal, CreatePipeline, CreateRevenue, CreateSource,
    DaemonDb, ExperimentFilter, ExperimentStatus, GoalFilter, GoalMetricType, GoalPeriod,
    GoalStatus, JobFilter, JobStatus, LogFilter, MetricType, ProposalFilter, ProposalStatus,
    RevenueFilter, RevenueSource, SourceType, Strategy,
};
use owo_colors::OwoColorize;
use std::path::PathBuf;

/// Daemon automation commands.
#[derive(Debug, Args)]
pub struct DaemonCli {
    /// Path to the daemon SQLite database.
    /// Defaults to ~/.jarvis/daemon.db
    #[clap(long, env = "JARVIS_DAEMON_DB")]
    db_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Show overall daemon status.
    Status,

    /// Manage pipelines.
    Pipeline(PipelineArgs),

    /// List recent jobs.
    Jobs(JobsArgs),

    /// Show daemon content.
    Content(ContentArgs),

    /// Show daemon logs.
    Logs(LogsArgs),

    /// Manage sources.
    Source(SourceArgs),

    /// View and manage proposals (actions suggested by the strategy analyzer).
    Proposals(ProposalsArgs),

    /// View revenue tracking and summaries.
    Revenue(RevenueArgs),

    /// Manage goals (measurable targets that drive the daemon strategy).
    Goals(GoalsArgs),

    /// View and manage A/B experiments.
    Experiments(ExperimentsArgs),

    /// View collected metrics.
    Metrics(MetricsArgs),

    /// Show daemon system health information.
    Health,
}

#[derive(Debug, Args)]
pub struct PipelineArgs {
    #[command(subcommand)]
    pub command: PipelineCommand,
}

#[derive(Debug, Subcommand)]
pub enum PipelineCommand {
    /// List all pipelines.
    List,
    /// Enable a pipeline.
    Enable { id: String },
    /// Disable a pipeline.
    Disable { id: String },
    /// Show pipeline config.
    Config { id: String },
    /// Add a new pipeline from a JSON file.
    Add {
        /// Path to a JSON config file.
        config_file: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct JobsArgs {
    /// Filter by pipeline ID.
    #[arg(long, short = 'p')]
    pipeline: Option<String>,
    /// Filter by status.
    #[arg(long, short = 's')]
    status: Option<String>,
    /// Max results.
    #[arg(long, short = 'n', default_value = "20")]
    limit: i64,
}

#[derive(Debug, Args)]
pub struct ContentArgs {
    /// Filter by pipeline ID.
    #[arg(long, short = 'p')]
    pipeline: Option<String>,
    /// Show content from the last N days.
    #[arg(long, default_value = "7")]
    last_days: i64,
    /// Max results.
    #[arg(long, short = 'n', default_value = "20")]
    limit: i64,
}

#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Filter by pipeline ID.
    #[arg(long, short = 'p')]
    pipeline: Option<String>,
    /// Filter by job ID.
    #[arg(long, short = 'j')]
    job: Option<String>,
    /// Max results.
    #[arg(long, short = 'n', default_value = "50")]
    limit: i64,
}

#[derive(Debug, Args)]
pub struct SourceArgs {
    #[command(subcommand)]
    pub command: SourceCommand,
}

#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    /// List sources for a pipeline.
    List {
        /// Pipeline ID.
        pipeline_id: String,
    },
    /// Add a source to a pipeline.
    Add {
        /// Pipeline ID.
        pipeline_id: String,
        /// Source type (rss, webpage, api, pdf_url).
        #[arg(long, short = 't')]
        source_type: String,
        /// Display name.
        #[arg(long)]
        name: String,
        /// URL to monitor.
        url: String,
        /// CSS selector (for webpage scraping).
        #[arg(long)]
        selector: Option<String>,
        /// Check interval in seconds.
        #[arg(long, default_value = "86400")]
        interval: i32,
    },
}

#[derive(Debug, Args)]
pub struct ProposalsArgs {
    #[command(subcommand)]
    pub command: ProposalsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProposalsCommand {
    /// List proposals (default: pending only).
    List {
        /// Show all proposals, not just pending.
        #[arg(long)]
        all: bool,
        /// Filter by pipeline ID.
        #[arg(long, short = 'p')]
        pipeline: Option<String>,
        /// Max results.
        #[arg(long, short = 'n', default_value = "20")]
        limit: i64,
    },
    /// Show details of a specific proposal.
    Show {
        /// Proposal ID (full or partial).
        id: String,
    },
    /// Approve a proposal for execution.
    Approve {
        /// Proposal ID.
        id: String,
    },
    /// Reject a proposal.
    Reject {
        /// Proposal ID.
        id: String,
        /// Reason for rejection (stored for learning).
        #[arg(long, short = 'r')]
        reason: Option<String>,
    },
    /// Expire stale proposals past their deadline.
    ExpireStale,
}

#[derive(Debug, Args)]
pub struct RevenueArgs {
    #[command(subcommand)]
    pub command: RevenueCommand,
}

#[derive(Debug, Subcommand)]
pub enum RevenueCommand {
    /// Show revenue summary.
    Summary {
        /// Period in days to summarize.
        #[arg(long, short = 'd', default_value = "30")]
        days: i64,
    },
    /// List individual revenue records.
    List {
        /// Filter by pipeline ID.
        #[arg(long, short = 'p')]
        pipeline: Option<String>,
        /// Show records from the last N days.
        #[arg(long, default_value = "30")]
        last_days: i64,
        /// Max results.
        #[arg(long, short = 'n', default_value = "50")]
        limit: i64,
    },
    /// Manually record a revenue entry.
    Add {
        /// Pipeline ID to attribute revenue to.
        pipeline_id: String,
        /// Revenue amount.
        amount: f64,
        /// Source of revenue (adsense, affiliate, gumroad, stripe, manual).
        #[arg(long, short = 's', default_value = "manual")]
        source: String,
        /// Currency code.
        #[arg(long, default_value = "USD")]
        currency: String,
        /// External transaction ID.
        #[arg(long)]
        external_id: Option<String>,
        /// Note / description.
        #[arg(long)]
        note: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct GoalsArgs {
    #[command(subcommand)]
    pub command: GoalsCommand,
}

#[derive(Debug, Subcommand)]
pub enum GoalsCommand {
    /// List goals.
    List {
        /// Show all goals (not just active).
        #[arg(long)]
        all: bool,
        /// Filter by pipeline ID.
        #[arg(long, short = 'p')]
        pipeline: Option<String>,
    },
    /// Add a new goal.
    Add {
        /// Goal name (e.g. "Monthly Revenue $50").
        name: String,
        /// Metric to track (revenue, content_count, pageviews, clicks, ctr, subscribers, cost_limit, custom).
        #[arg(long, short = 'm')]
        metric: String,
        /// Target value.
        #[arg(long, short = 't')]
        target: f64,
        /// Period (daily, weekly, monthly, quarterly, yearly).
        #[arg(long, default_value = "monthly")]
        period: String,
        /// Unit label (USD, articles, etc.).
        #[arg(long, default_value = "USD")]
        unit: String,
        /// Pipeline to scope this goal to (optional).
        #[arg(long, short = 'p')]
        pipeline: Option<String>,
        /// Priority (lower = higher priority, default 1).
        #[arg(long, default_value = "1")]
        priority: i32,
        /// Description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Show goal progress.
    Progress,
    /// Pause a goal.
    Pause {
        /// Goal ID (full or partial).
        id: String,
    },
    /// Resume a paused goal.
    Resume {
        /// Goal ID (full or partial).
        id: String,
    },
    /// Archive a goal.
    Archive {
        /// Goal ID (full or partial).
        id: String,
    },
}

#[derive(Debug, Args)]
pub struct ExperimentsArgs {
    #[command(subcommand)]
    pub command: ExperimentsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ExperimentsCommand {
    /// List experiments.
    List {
        /// Filter by pipeline ID.
        #[arg(long, short = 'p')]
        pipeline: Option<String>,
        /// Show all experiments (not just running).
        #[arg(long)]
        all: bool,
        /// Max results.
        #[arg(long, short = 'n', default_value = "20")]
        limit: i64,
    },
    /// Show experiment details.
    Show {
        /// Experiment ID (full or partial).
        id: String,
    },
    /// Cancel a running experiment.
    Cancel {
        /// Experiment ID (full or partial).
        id: String,
    },
}

#[derive(Debug, Args)]
pub struct MetricsArgs {
    #[command(subcommand)]
    pub command: MetricsCommand,
}

#[derive(Debug, Subcommand)]
pub enum MetricsCommand {
    /// Show metrics summary for a period.
    Summary {
        /// Period in days.
        #[arg(long, short = 'd', default_value = "30")]
        days: i64,
    },
    /// Show per-content metrics.
    Content {
        /// Content ID.
        content_id: String,
    },
}

impl DaemonCli {
    fn db_path(&self) -> PathBuf {
        self.db_path.clone().unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".jarvis").join("daemon.db")
        })
    }
}

/// Run the daemon subcommand.
pub async fn run_daemon_command(cli: DaemonCli) -> Result<()> {
    let db_path = cli.db_path();

    // Ensure parent directory exists.
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let db = DaemonDb::open(&db_path).await?;

    match cli.command {
        DaemonCommand::Status => cmd_status(&db).await,
        DaemonCommand::Pipeline(args) => cmd_pipeline(&db, args).await,
        DaemonCommand::Jobs(args) => cmd_jobs(&db, args).await,
        DaemonCommand::Content(args) => cmd_content(&db, args).await,
        DaemonCommand::Logs(args) => cmd_logs(&db, args).await,
        DaemonCommand::Source(args) => cmd_source(&db, args).await,
        DaemonCommand::Proposals(args) => cmd_proposals(&db, args).await,
        DaemonCommand::Revenue(args) => cmd_revenue(&db, args).await,
        DaemonCommand::Goals(args) => cmd_goals(&db, args).await,
        DaemonCommand::Experiments(args) => cmd_experiments(&db, args).await,
        DaemonCommand::Metrics(args) => cmd_metrics(&db, args).await,
        DaemonCommand::Health => cmd_health(&db).await,
    }
}

async fn cmd_status(db: &DaemonDb) -> Result<()> {
    let all_pipelines = db.list_pipelines(false).await?;
    let enabled = all_pipelines.iter().filter(|p| p.enabled).count();
    let running_jobs = db.count_running_jobs().await?;
    let pending_proposals = db.count_pending_proposals().await?;

    let content_filter = ContentFilter {
        since_days: Some(7),
        status: Some(ContentStatus::Published),
        ..Default::default()
    };
    let recent_content = db.list_content(&content_filter).await?;
    let revenue_summary = db.revenue_summary(30).await?;

    println!("{}", "=== Jarvis Daemon Status ===".bold());
    println!();
    println!(
        "Pipelines:      {} total, {} enabled",
        all_pipelines.len().to_string().cyan(),
        enabled.to_string().green()
    );
    println!("Running jobs:   {}", running_jobs.to_string().yellow());
    println!(
        "Published (7d): {} items",
        recent_content.len().to_string().green()
    );
    println!(
        "Proposals:      {} pending",
        if pending_proposals > 0 {
            pending_proposals.to_string().yellow().to_string()
        } else {
            "0".dimmed().to_string()
        }
    );
    println!("Revenue (30d):  ${:.2}", revenue_summary.total_usd);
    println!();

    if !all_pipelines.is_empty() {
        println!("{}", "Pipelines:".bold());
        for p in &all_pipelines {
            let status = if p.enabled {
                "enabled".green().to_string()
            } else {
                "disabled".red().to_string()
            };
            println!(
                "  {} [{}] {} ({})",
                p.id.cyan(),
                status,
                p.name,
                p.schedule_cron.dimmed()
            );
        }
    }

    // Show active goals summary.
    let active_goals = db
        .list_goals(&GoalFilter {
            status: Some(GoalStatus::Active),
            ..Default::default()
        })
        .await?;
    if !active_goals.is_empty() {
        println!();
        println!("{}", "Active Goals:".bold());
        for g in &active_goals {
            let pct = if g.target_value > 0.0 {
                (g.current_value / g.target_value) * 100.0
            } else {
                0.0
            };
            let bar = progress_bar(pct, 20);
            let status_label = if pct >= 100.0 {
                "DONE".green().to_string()
            } else if pct >= 60.0 {
                "OK".cyan().to_string()
            } else {
                "RISK".yellow().to_string()
            };
            println!(
                "  P{} {} {:.1}/{:.1} {} {} [{status_label}]",
                g.priority,
                g.name.bold(),
                g.current_value,
                g.target_value,
                g.target_unit,
                bar,
            );
        }
    }

    if pending_proposals > 0 {
        println!();
        println!(
            "{} Use '{}' to review.",
            "Pending proposals!".yellow().bold(),
            "jarvis daemon proposals list".cyan()
        );
    }

    Ok(())
}

async fn cmd_pipeline(db: &DaemonDb, args: PipelineArgs) -> Result<()> {
    match args.command {
        PipelineCommand::List => {
            let pipelines = db.list_pipelines(false).await?;
            if pipelines.is_empty() {
                println!("No pipelines registered.");
                println!("Use 'jarvis daemon pipeline add <config.json>' to add one.");
                return Ok(());
            }

            println!(
                "{:<25} {:<20} {:<12} {:<15} {}",
                "ID".bold(),
                "Name".bold(),
                "Strategy".bold(),
                "Schedule".bold(),
                "Status".bold()
            );
            for p in &pipelines {
                let status = if p.enabled { "enabled" } else { "disabled" };
                println!(
                    "{:<25} {:<20} {:<12} {:<15} {}",
                    p.id, p.name, p.strategy, p.schedule_cron, status
                );
            }
            Ok(())
        }
        PipelineCommand::Enable { id } => {
            db.set_pipeline_enabled(&id, true).await?;
            println!("Pipeline '{}' enabled.", id.green());
            Ok(())
        }
        PipelineCommand::Disable { id } => {
            db.set_pipeline_enabled(&id, false).await?;
            println!("Pipeline '{}' disabled.", id.yellow());
            Ok(())
        }
        PipelineCommand::Config { id } => {
            let pipeline = db
                .get_pipeline(&id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("pipeline '{id}' not found"))?;
            let config: serde_json::Value = serde_json::from_str(&pipeline.config_json)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
            Ok(())
        }
        PipelineCommand::Add { config_file } => {
            let contents = tokio::fs::read_to_string(&config_file).await?;
            let config: serde_json::Value = serde_json::from_str(&contents)?;

            let input = CreatePipeline {
                id: config["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("config must have 'id' field"))?
                    .to_string(),
                name: config["name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("config must have 'name' field"))?
                    .to_string(),
                strategy: config["strategy"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("config must have 'strategy' field"))?
                    .parse::<Strategy>()?,
                config_json: config.clone(),
                schedule_cron: config["schedule_cron"]
                    .as_str()
                    .unwrap_or("0 3 * * *")
                    .to_string(),
                max_retries: config["max_retries"].as_i64().map(|v| v as i32),
                retry_delay_sec: config["retry_delay_sec"].as_i64().map(|v| v as i32),
            };

            let pipeline = db.create_pipeline(&input).await?;
            println!("Pipeline '{}' created successfully.", pipeline.id.green());
            Ok(())
        }
    }
}

async fn cmd_jobs(db: &DaemonDb, args: JobsArgs) -> Result<()> {
    let status = args
        .status
        .as_deref()
        .map(|s| match s {
            "pending" => Ok(JobStatus::Pending),
            "running" => Ok(JobStatus::Running),
            "completed" => Ok(JobStatus::Completed),
            "failed" => Ok(JobStatus::Failed),
            "cancelled" => Ok(JobStatus::Cancelled),
            other => Err(anyhow::anyhow!("unknown status: {other}")),
        })
        .transpose()?;

    let filter = JobFilter {
        pipeline_id: args.pipeline,
        status,
        limit: Some(args.limit),
    };

    let jobs = db.list_jobs(&filter).await?;
    if jobs.is_empty() {
        println!("No jobs found.");
        return Ok(());
    }

    println!(
        "{:<38} {:<20} {:<12} {:<12} {}",
        "ID".bold(),
        "Pipeline".bold(),
        "Status".bold(),
        "Duration".bold(),
        "Created".bold()
    );

    for job in &jobs {
        let duration = job
            .duration_ms
            .map(|ms| format!("{}ms", ms))
            .unwrap_or_else(|| "-".to_string());
        let created = format_timestamp(job.created_at);

        println!(
            "{:<38} {:<20} {:<12} {:<12} {}",
            &job.id[..8],
            job.pipeline_id,
            job.status,
            duration,
            created
        );
    }

    Ok(())
}

async fn cmd_content(db: &DaemonDb, args: ContentArgs) -> Result<()> {
    let filter = ContentFilter {
        pipeline_id: args.pipeline,
        since_days: Some(args.last_days),
        limit: Some(args.limit),
        ..Default::default()
    };

    let content = db.list_content(&filter).await?;
    if content.is_empty() {
        println!("No content found in the last {} days.", args.last_days);
        return Ok(());
    }

    println!(
        "{:<38} {:<12} {:<12} {:<10} {}",
        "Title".bold(),
        "Platform".bold(),
        "Status".bold(),
        "Words".bold(),
        "Published".bold()
    );

    for item in &content {
        let title = item
            .title
            .as_deref()
            .unwrap_or("(untitled)")
            .chars()
            .take(36)
            .collect::<String>();
        let words = item
            .word_count
            .map(|w| w.to_string())
            .unwrap_or_else(|| "-".to_string());
        let published = item
            .published_at
            .map(format_timestamp)
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<38} {:<12} {:<12} {:<10} {}",
            title, item.platform, item.status, words, published
        );
    }

    Ok(())
}

async fn cmd_logs(db: &DaemonDb, args: LogsArgs) -> Result<()> {
    let filter = LogFilter {
        pipeline_id: args.pipeline,
        job_id: args.job,
        limit: Some(args.limit),
        ..Default::default()
    };

    let logs = db.list_logs(&filter).await?;
    if logs.is_empty() {
        println!("No logs found.");
        return Ok(());
    }

    for log in &logs {
        let time = format_timestamp(log.created_at);
        let level_colored = match log.level.as_str() {
            "error" => log.level.red().to_string(),
            "warn" => log.level.yellow().to_string(),
            "info" => log.level.green().to_string(),
            "debug" => log.level.dimmed().to_string(),
            _ => log.level.clone(),
        };

        println!(
            "{} [{}] {} {}",
            time.dimmed(),
            level_colored,
            log.pipeline_id.cyan(),
            log.message
        );
    }

    Ok(())
}

async fn cmd_source(db: &DaemonDb, args: SourceArgs) -> Result<()> {
    match args.command {
        SourceCommand::List { pipeline_id } => {
            let sources = db.list_sources(&pipeline_id).await?;
            if sources.is_empty() {
                println!("No sources for pipeline '{pipeline_id}'.");
                return Ok(());
            }

            println!(
                "{:<15} {:<25} {:<8} {:<12} {}",
                "Type".bold(),
                "Name".bold(),
                "Enabled".bold(),
                "Last Check".bold(),
                "URL".bold()
            );

            for src in &sources {
                let last_check = src
                    .last_checked_at
                    .map(format_timestamp)
                    .unwrap_or_else(|| "never".to_string());
                let enabled = if src.enabled { "yes" } else { "no" };

                println!(
                    "{:<15} {:<25} {:<8} {:<12} {}",
                    src.source_type, src.name, enabled, last_check, src.url
                );
            }
            Ok(())
        }
        SourceCommand::Add {
            pipeline_id,
            source_type,
            name,
            url,
            selector,
            interval,
        } => {
            let st = match source_type.as_str() {
                "rss" => SourceType::Rss,
                "webpage" => SourceType::Webpage,
                "api" => SourceType::Api,
                "pdf_url" => SourceType::PdfUrl,
                other => anyhow::bail!("unknown source type: {other}"),
            };

            let input = CreateSource {
                pipeline_id: pipeline_id.clone(),
                source_type: st,
                name: name.clone(),
                url: url.clone(),
                scrape_selector: selector,
                check_interval_sec: Some(interval),
            };

            let source = db.create_source(&input).await?;
            println!(
                "Source '{}' added to pipeline '{}'.",
                source.name.green(),
                pipeline_id
            );
            Ok(())
        }
    }
}

async fn cmd_proposals(db: &DaemonDb, args: ProposalsArgs) -> Result<()> {
    match args.command {
        ProposalsCommand::List {
            all,
            pipeline,
            limit,
        } => {
            let filter = ProposalFilter {
                pipeline_id: pipeline,
                status: if all {
                    None
                } else {
                    Some(ProposalStatus::Pending)
                },
                limit: Some(limit),
                ..Default::default()
            };

            let proposals = db.list_proposals(&filter).await?;
            if proposals.is_empty() {
                if all {
                    println!("No proposals found.");
                } else {
                    println!("No pending proposals. Use {} to see all.", "--all".cyan());
                }
                return Ok(());
            }

            println!(
                "{:<10} {:<14} {:<8} {:<10} {:<7} {}",
                "ID".bold(),
                "Action".bold(),
                "Risk".bold(),
                "Status".bold(),
                "Conf.".bold(),
                "Title".bold()
            );

            for p in &proposals {
                let id_short = if p.id.len() > 8 { &p.id[..8] } else { &p.id };
                let risk_colored = match p.risk_level.as_str() {
                    "high" => p.risk_level.red().to_string(),
                    "medium" => p.risk_level.yellow().to_string(),
                    "low" => p.risk_level.green().to_string(),
                    _ => p.risk_level.clone(),
                };
                let status_colored = match p.status.as_str() {
                    "pending" => p.status.yellow().to_string(),
                    "approved" => p.status.green().to_string(),
                    "rejected" => p.status.red().to_string(),
                    "executed" => p.status.cyan().to_string(),
                    "expired" => p.status.dimmed().to_string(),
                    "failed" => p.status.red().bold().to_string(),
                    _ => p.status.clone(),
                };
                let title_short: String = p.title.chars().take(40).collect();

                println!(
                    "{:<10} {:<14} {:<8} {:<10} {:<7.0}% {}",
                    id_short,
                    p.action_type,
                    risk_colored,
                    status_colored,
                    p.confidence * 100.0,
                    title_short
                );
            }

            Ok(())
        }
        ProposalsCommand::Show { id } => {
            // Find proposal by partial ID match.
            let filter = ProposalFilter::default();
            let all = db.list_proposals(&filter).await?;
            let proposal = all
                .iter()
                .find(|p| p.id.starts_with(&id))
                .ok_or_else(|| anyhow::anyhow!("no proposal found matching '{id}'"))?;

            println!("{}", "=== Proposal Details ===".bold());
            println!();
            println!("ID:          {}", proposal.id.cyan());
            println!("Action:      {}", proposal.action_type);
            if let Some(ref pid) = proposal.pipeline_id {
                println!("Pipeline:    {}", pid);
            }
            println!("Title:       {}", proposal.title.bold());
            println!(
                "Risk:        {}",
                match proposal.risk_level.as_str() {
                    "high" => proposal.risk_level.red().to_string(),
                    "medium" => proposal.risk_level.yellow().to_string(),
                    "low" => proposal.risk_level.green().to_string(),
                    _ => proposal.risk_level.clone(),
                }
            );
            println!("Confidence:  {:.1}%", proposal.confidence * 100.0);
            println!("Status:      {}", proposal.status);
            println!(
                "Auto-approve: {}",
                if proposal.auto_approvable {
                    "yes".green().to_string()
                } else {
                    "no".dimmed().to_string()
                }
            );
            println!("Created:     {}", format_timestamp(proposal.created_at));
            if let Some(ts) = proposal.reviewed_at {
                println!("Reviewed:    {}", format_timestamp(ts));
            }
            if let Some(ts) = proposal.executed_at {
                println!("Executed:    {}", format_timestamp(ts));
            }
            if let Some(ts) = proposal.expires_at {
                println!("Expires:     {}", format_timestamp(ts));
            }
            println!();
            println!("{}", "Description:".bold());
            println!("  {}", proposal.description);
            println!();
            println!("{}", "LLM Reasoning:".bold());
            println!("  {}", proposal.reasoning);

            if let Some(ref config) = proposal.proposed_config {
                println!();
                println!("{}", "Proposed Config:".bold());
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(config) {
                    println!("  {}", serde_json::to_string_pretty(&parsed)?);
                } else {
                    println!("  {config}");
                }
            }

            if proposal.status == "pending" {
                println!();
                println!(
                    "Actions: {} or {}",
                    format!("jarvis daemon proposals approve {}", &proposal.id[..8]).green(),
                    format!("jarvis daemon proposals reject {}", &proposal.id[..8]).red()
                );
            }

            Ok(())
        }
        ProposalsCommand::Approve { id } => {
            // Find by partial ID.
            let filter = ProposalFilter {
                status: Some(ProposalStatus::Pending),
                ..Default::default()
            };
            let pending = db.list_proposals(&filter).await?;
            let proposal = pending
                .iter()
                .find(|p| p.id.starts_with(&id))
                .ok_or_else(|| anyhow::anyhow!("no pending proposal found matching '{id}'"))?;

            db.approve_proposal(&proposal.id).await?;
            let id_short: String = proposal.id.chars().take(8).collect();
            println!("Proposal {} approved: {}", id_short.green(), proposal.title);
            println!("It will be executed on the next scheduler tick.");
            Ok(())
        }
        ProposalsCommand::Reject { id, reason } => {
            let filter = ProposalFilter {
                status: Some(ProposalStatus::Pending),
                ..Default::default()
            };
            let pending = db.list_proposals(&filter).await?;
            let proposal = pending
                .iter()
                .find(|p| p.id.starts_with(&id))
                .ok_or_else(|| anyhow::anyhow!("no pending proposal found matching '{id}'"))?;

            db.reject_proposal(&proposal.id).await?;
            let id_short: String = proposal.id.chars().take(8).collect();
            println!("Proposal {} rejected: {}", id_short.red(), proposal.title);
            if let Some(reason) = reason {
                println!("Reason: {reason}");
            }
            Ok(())
        }
        ProposalsCommand::ExpireStale => {
            let count = db.expire_proposals().await?;
            if count > 0 {
                println!("Expired {} stale proposals.", count.to_string().yellow());
            } else {
                println!("No stale proposals to expire.");
            }
            Ok(())
        }
    }
}

async fn cmd_revenue(db: &DaemonDb, args: RevenueArgs) -> Result<()> {
    match args.command {
        RevenueCommand::Add {
            pipeline_id,
            amount,
            source,
            currency,
            external_id,
            note,
        } => {
            let rev_source: RevenueSource = source
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid source: {source}"))?;
            let now = chrono::Utc::now().timestamp();
            let metadata = note.map(|n| serde_json::json!({"note": n}));

            let input = CreateRevenue {
                content_id: None,
                pipeline_id: pipeline_id.clone(),
                source: rev_source,
                amount,
                currency: Some(currency.clone()),
                period_start: now,
                period_end: now,
                external_id,
                metadata,
            };

            let revenue = db.create_revenue(&input).await?;
            println!(
                "Revenue recorded: {} {} from {} (pipeline: {})",
                format!("${:.2}", revenue.amount).green().bold(),
                revenue.currency,
                revenue.source,
                pipeline_id
            );
            println!("ID: {}", revenue.id.dimmed());
            Ok(())
        }
        RevenueCommand::Summary { days } => {
            let summary = db.revenue_summary(days).await?;

            println!("{}", format!("=== Revenue Summary ({days}d) ===").bold());
            println!();
            println!(
                "Total:  {}",
                format!("${:.2}", summary.total_usd).green().bold()
            );
            println!("Period: {} days", summary.period_days);
            println!();

            if !summary.by_pipeline.is_empty() {
                println!("{}", "By Pipeline:".bold());
                for pr in &summary.by_pipeline {
                    let name = pr.pipeline_name.as_deref().unwrap_or(&pr.pipeline_id);
                    println!(
                        "  {:<30} ${:<10.2} ({} records)",
                        name, pr.total_usd, pr.content_count
                    );
                }
                println!();
            }

            if !summary.by_source.is_empty() {
                println!("{}", "By Source:".bold());
                for sr in &summary.by_source {
                    println!(
                        "  {:<15} ${:<10.2} ({} records)",
                        sr.source, sr.total_usd, sr.record_count
                    );
                }
            }

            if summary.total_usd == 0.0 {
                println!();
                println!(
                    "{}",
                    "No revenue recorded yet. The metrics_collector pipeline will start tracking revenue automatically.".dimmed()
                );
            }

            Ok(())
        }
        RevenueCommand::List {
            pipeline,
            last_days,
            limit,
        } => {
            let filter = RevenueFilter {
                pipeline_id: pipeline,
                since_days: Some(last_days),
                limit: Some(limit),
                ..Default::default()
            };

            let records = db.list_revenue(&filter).await?;
            if records.is_empty() {
                println!("No revenue records found in the last {last_days} days.");
                return Ok(());
            }

            println!(
                "{:<10} {:<15} {:<12} {:<10} {:<15} {}",
                "ID".bold(),
                "Source".bold(),
                "Amount".bold(),
                "Currency".bold(),
                "Period Start".bold(),
                "Pipeline".bold()
            );

            for r in &records {
                let id_short = if r.id.len() > 8 { &r.id[..8] } else { &r.id };
                println!(
                    "{:<10} {:<15} ${:<11.2} {:<10} {:<15} {}",
                    id_short,
                    r.source,
                    r.amount,
                    r.currency,
                    format_timestamp(r.period_start),
                    r.pipeline_id
                );
            }

            Ok(())
        }
    }
}

async fn cmd_goals(db: &DaemonDb, args: GoalsArgs) -> Result<()> {
    match args.command {
        GoalsCommand::List { all, pipeline } => {
            let filter = GoalFilter {
                status: if all { None } else { Some(GoalStatus::Active) },
                pipeline_id: pipeline,
                ..Default::default()
            };
            let goals = db.list_goals(&filter).await?;
            if goals.is_empty() {
                if all {
                    println!("No goals found.");
                } else {
                    println!(
                        "No active goals. Use {} to add one, or {} to see all.",
                        "jarvis daemon goals add".cyan(),
                        "--all".cyan()
                    );
                }
                return Ok(());
            }

            println!(
                "{:<10} {:<4} {:<30} {:<15} {:<20} {:<8} {}",
                "ID".bold(),
                "P".bold(),
                "Name".bold(),
                "Metric".bold(),
                "Progress".bold(),
                "Status".bold(),
                "Period".bold()
            );

            for g in &goals {
                let id_short = if g.id.len() > 8 { &g.id[..8] } else { &g.id };
                let pct = if g.target_value > 0.0 {
                    (g.current_value / g.target_value) * 100.0
                } else {
                    0.0
                };
                let bar = progress_bar(pct, 12);
                let progress_str = format!(
                    "{:.1}/{:.1} {} {bar}",
                    g.current_value, g.target_value, g.target_unit
                );
                let status_colored = match g.status.as_str() {
                    "active" => g.status.green().to_string(),
                    "achieved" => g.status.cyan().to_string(),
                    "paused" => g.status.yellow().to_string(),
                    "failed" => g.status.red().to_string(),
                    "archived" => g.status.dimmed().to_string(),
                    _ => g.status.clone(),
                };
                let name_short: String = g.name.chars().take(28).collect();

                println!(
                    "{:<10} {:<4} {:<30} {:<15} {:<20} {:<8} {}",
                    id_short,
                    g.priority,
                    name_short,
                    g.metric_type,
                    progress_str,
                    status_colored,
                    g.period
                );
            }

            Ok(())
        }
        GoalsCommand::Add {
            name,
            metric,
            target,
            period,
            unit,
            pipeline,
            priority,
            description,
        } => {
            let metric_type: GoalMetricType = metric
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid metric type: {metric}"))?;
            let goal_period: GoalPeriod = period
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid period: {period}"))?;

            let input = CreateGoal {
                name: name.clone(),
                description,
                metric_type,
                target_value: target,
                target_unit: Some(unit),
                period: goal_period,
                pipeline_id: pipeline,
                priority: Some(priority),
                deadline: None,
            };

            let goal = db.create_goal(&input).await?;
            println!(
                "Goal created: {} ({} {} per {})",
                goal.name.green().bold(),
                goal.target_value,
                goal.target_unit,
                goal.period
            );
            println!("ID: {}", goal.id.dimmed());
            Ok(())
        }
        GoalsCommand::Progress => {
            let goals = db
                .list_goals(&GoalFilter {
                    status: Some(GoalStatus::Active),
                    ..Default::default()
                })
                .await?;

            if goals.is_empty() {
                println!("No active goals.");
                return Ok(());
            }

            println!("{}", "=== Goal Progress ===".bold());
            println!();

            for g in &goals {
                let pct = if g.target_value > 0.0 {
                    (g.current_value / g.target_value) * 100.0
                } else {
                    0.0
                };
                let gap = g.target_value - g.current_value;
                let bar = progress_bar(pct, 30);
                let status_label = if pct >= 100.0 {
                    "ACHIEVED".green().bold().to_string()
                } else if pct >= 60.0 {
                    "ON TRACK".cyan().to_string()
                } else {
                    "AT RISK".yellow().bold().to_string()
                };

                println!("  P{} {} [{}]", g.priority, g.name.bold(), status_label);
                println!(
                    "     {:.1}/{:.1} {} ({:.0}%) {bar}",
                    g.current_value, g.target_value, g.target_unit, pct
                );
                if gap > 0.0 {
                    println!("     Gap: {:.2} {} remaining", gap, g.target_unit.dimmed());
                }
                if let Some(ts) = g.last_measured {
                    println!("     Last measured: {}", format_timestamp(ts).dimmed());
                }
                println!();
            }

            Ok(())
        }
        GoalsCommand::Pause { id } => {
            let goal = find_goal(db, &id).await?;
            db.set_goal_status(&goal.id, GoalStatus::Paused).await?;
            println!("Goal '{}' paused.", goal.name.yellow());
            Ok(())
        }
        GoalsCommand::Resume { id } => {
            let goal = find_goal(db, &id).await?;
            db.set_goal_status(&goal.id, GoalStatus::Active).await?;
            println!("Goal '{}' resumed.", goal.name.green());
            Ok(())
        }
        GoalsCommand::Archive { id } => {
            let goal = find_goal(db, &id).await?;
            db.set_goal_status(&goal.id, GoalStatus::Archived).await?;
            println!("Goal '{}' archived.", goal.name.dimmed());
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Experiments subcommand
// ---------------------------------------------------------------------------

async fn cmd_experiments(db: &DaemonDb, args: ExperimentsArgs) -> Result<()> {
    match args.command {
        ExperimentsCommand::List {
            pipeline,
            all,
            limit,
        } => {
            let filter = ExperimentFilter {
                pipeline_id: pipeline,
                status: if all {
                    None
                } else {
                    Some(ExperimentStatus::Running)
                },
                limit: Some(limit),
                ..Default::default()
            };

            let experiments = db.list_experiments(&filter).await?;
            if experiments.is_empty() {
                if all {
                    println!("No experiments found.");
                } else {
                    println!("No running experiments. Use {} to see all.", "--all".cyan());
                }
                return Ok(());
            }

            println!(
                "{:<10} {:<10} {:<10} {:<12} {:<8} {:<8} {}",
                "ID".bold(),
                "Type".bold(),
                "Status".bold(),
                "Metric".bold(),
                "A".bold(),
                "B".bold(),
                "Winner".bold()
            );

            for exp in &experiments {
                let id_short = if exp.id.len() > 8 {
                    &exp.id[..8]
                } else {
                    &exp.id
                };
                let status_colored = match exp.status.as_str() {
                    "running" => exp.status.yellow().to_string(),
                    "completed" => exp.status.green().to_string(),
                    "cancelled" => exp.status.dimmed().to_string(),
                    _ => exp.status.clone(),
                };
                let winner = exp
                    .winner
                    .as_deref()
                    .map(|w| w.to_uppercase())
                    .unwrap_or_else(|| "-".to_string());

                println!(
                    "{:<10} {:<10} {:<10} {:<12} {:<8.2} {:<8.2} {}",
                    id_short,
                    exp.experiment_type,
                    status_colored,
                    exp.metric,
                    exp.metric_a,
                    exp.metric_b,
                    winner,
                );
            }

            Ok(())
        }
        ExperimentsCommand::Show { id } => {
            let filter = ExperimentFilter::default();
            let all = db.list_experiments(&filter).await?;
            let exp = all
                .iter()
                .find(|e| e.id.starts_with(&id))
                .ok_or_else(|| anyhow::anyhow!("no experiment found matching '{id}'"))?;

            println!("{}", "=== Experiment Details ===".bold());
            println!();
            println!("ID:            {}", exp.id.cyan());
            println!("Type:          {}", exp.experiment_type);
            println!("Pipeline:      {}", exp.pipeline_id);
            println!("Content:       {}", exp.content_id);
            println!("Status:        {}", exp.status);
            println!("Metric:        {}", exp.metric);
            println!(
                "Active:        Variant {}",
                exp.active_variant.to_uppercase()
            );
            println!();
            println!("{}", "Variants:".bold());
            println!("  A: {}", exp.variant_a);
            println!("  B: {}", exp.variant_b);
            println!();
            println!("{}", "Performance:".bold());
            println!("  A metric:  {:.4}", exp.metric_a);
            println!("  B metric:  {:.4}", exp.metric_b);
            if let Some(ref w) = exp.winner {
                println!(
                    "  Winner:    {}",
                    format!("Variant {}", w.to_uppercase()).green().bold()
                );
            }
            println!();
            println!("Min duration:  {} days", exp.min_duration_days);
            println!("Created:       {}", format_timestamp(exp.created_at));
            println!("Updated:       {}", format_timestamp(exp.updated_at));
            if let Some(ts) = exp.completed_at {
                println!("Completed:     {}", format_timestamp(ts));
            }

            if exp.status == "running" {
                let id_short: String = exp.id.chars().take(8).collect();
                println!();
                println!(
                    "Cancel: {}",
                    format!("jarvis daemon experiments cancel {id_short}").red()
                );
            }

            Ok(())
        }
        ExperimentsCommand::Cancel { id } => {
            let filter = ExperimentFilter {
                status: Some(ExperimentStatus::Running),
                ..Default::default()
            };
            let running = db.list_experiments(&filter).await?;
            let exp = running
                .iter()
                .find(|e| e.id.starts_with(&id))
                .ok_or_else(|| anyhow::anyhow!("no running experiment found matching '{id}'"))?;

            db.cancel_experiment(&exp.id).await?;
            let id_short: String = exp.id.chars().take(8).collect();
            println!(
                "Experiment {} cancelled. Active variant '{}' will remain.",
                id_short.yellow(),
                exp.active_variant,
            );
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Metrics subcommand
// ---------------------------------------------------------------------------

async fn cmd_metrics(db: &DaemonDb, args: MetricsArgs) -> Result<()> {
    match args.command {
        MetricsCommand::Summary { days } => {
            let now = chrono::Utc::now().timestamp();
            let since = now - (days * 86400);

            let views = db
                .sum_metrics(MetricType::Views, since, None)
                .await
                .unwrap_or(0.0);
            let clicks = db
                .sum_metrics(MetricType::Clicks, since, None)
                .await
                .unwrap_or(0.0);
            let impressions = db
                .sum_metrics(MetricType::Impressions, since, None)
                .await
                .unwrap_or(0.0);
            let ctr = if impressions > 0.0 {
                (clicks / impressions) * 100.0
            } else {
                0.0
            };
            let revenue = db.revenue_summary(days).await?;

            println!("{}", format!("=== Metrics Summary ({days}d) ===").bold());
            println!();
            println!("  Views:        {}", format!("{:.0}", views).cyan().bold());
            println!(
                "  Clicks:       {}",
                format!("{:.0}", clicks).green().bold()
            );
            println!("  Impressions:  {}", format!("{:.0}", impressions).cyan());
            println!("  CTR:          {}", format!("{:.2}%", ctr).yellow());
            println!(
                "  Revenue:      {}",
                format!("${:.2}", revenue.total_usd).green().bold()
            );
            println!();

            // Show per-source breakdown.
            let sources = [
                "wordpress",
                "google_search_console",
                "google_adsense",
                "google_analytics",
            ];
            let source_labels = ["WordPress", "Search Console", "AdSense", "Analytics 4"];
            let mut has_source_data = false;

            for (src, label) in sources.iter().zip(source_labels.iter()) {
                let src_views = db
                    .sum_metrics(MetricType::Views, since, Some(src))
                    .await
                    .unwrap_or(0.0);
                let src_clicks = db
                    .sum_metrics(MetricType::Clicks, since, Some(src))
                    .await
                    .unwrap_or(0.0);

                if src_views > 0.0 || src_clicks > 0.0 {
                    if !has_source_data {
                        println!("{}", "By Source:".bold());
                        has_source_data = true;
                    }
                    println!(
                        "  {:<20} views: {:<10.0} clicks: {:.0}",
                        label, src_views, src_clicks
                    );
                }
            }

            if !has_source_data {
                println!(
                    "{}",
                    "No metrics data yet. Run the metrics_collector pipeline to start gathering data.".dimmed()
                );
            }

            Ok(())
        }
        MetricsCommand::Content { content_id } => {
            let views = db.sum_content_metric(&content_id, "views").await?;
            let clicks = db.sum_content_metric(&content_id, "clicks").await?;
            let impressions = db.sum_content_metric(&content_id, "impressions").await?;
            let revenue = db.sum_content_metric(&content_id, "revenue").await?;
            let ctr = db.sum_content_metric(&content_id, "ctr").await?;

            let id_short = if content_id.len() > 8 {
                &content_id[..8]
            } else {
                &content_id
            };

            println!(
                "{}",
                format!("=== Metrics for content {id_short} ===").bold()
            );
            println!();
            println!("  Views:        {:.0}", views);
            println!("  Clicks:       {:.0}", clicks);
            println!("  Impressions:  {:.0}", impressions);
            println!("  CTR:          {:.4}", ctr);
            println!("  Revenue:      ${:.2}", revenue);

            if views == 0.0 && clicks == 0.0 && impressions == 0.0 {
                println!();
                println!("{}", "No metrics recorded for this content yet.".dimmed());
            }

            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Health subcommand
// ---------------------------------------------------------------------------

async fn cmd_health(db: &DaemonDb) -> Result<()> {
    let pipelines = db.list_pipelines(false).await?;
    let enabled = pipelines.iter().filter(|p| p.enabled).count();
    let running_jobs = db.count_running_jobs().await?;
    let pending_proposals = db.count_pending_proposals().await?;

    // Recent job stats (last 24h).
    let job_filter_ok = JobFilter {
        status: Some(JobStatus::Completed),
        ..Default::default()
    };
    let completed_jobs = db.list_jobs(&job_filter_ok).await?;
    let now = chrono::Utc::now().timestamp();
    let last_24h = now - 86400;
    let completed_24h = completed_jobs
        .iter()
        .filter(|j| j.created_at > last_24h)
        .count();

    let job_filter_fail = JobFilter {
        status: Some(JobStatus::Failed),
        ..Default::default()
    };
    let failed_jobs = db.list_jobs(&job_filter_fail).await?;
    let failed_24h = failed_jobs
        .iter()
        .filter(|j| j.created_at > last_24h)
        .count();

    // Active goals status.
    let goals = db
        .list_goals(&GoalFilter {
            status: Some(GoalStatus::Active),
            ..Default::default()
        })
        .await?;
    let at_risk = goals
        .iter()
        .filter(|g| {
            if g.target_value <= 0.0 {
                return false;
            }
            (g.current_value / g.target_value) < 0.4
        })
        .count();

    // Experiments.
    let exp_filter = ExperimentFilter {
        status: Some(ExperimentStatus::Running),
        ..Default::default()
    };
    let running_experiments = db.list_experiments(&exp_filter).await?.len();

    // Revenue.
    let revenue = db.revenue_summary(30).await?;

    // Overall health assessment.
    let health = if failed_24h > completed_24h && completed_24h > 0 {
        "DEGRADED".red().bold().to_string()
    } else if enabled == 0 {
        "INACTIVE".yellow().to_string()
    } else if at_risk > 0 {
        "AT RISK".yellow().bold().to_string()
    } else {
        "HEALTHY".green().bold().to_string()
    };

    println!("{}", "=== Jarvis Daemon Health ===".bold());
    println!();
    println!("  Status:             {health}");
    println!();
    println!("{}", "  System:".bold());
    println!(
        "    Pipelines:        {enabled}/{} enabled",
        pipelines.len()
    );
    println!("    Running jobs:     {running_jobs}");
    println!(
        "    Jobs (24h):       {} completed, {} failed",
        completed_24h, failed_24h
    );
    println!();
    println!("{}", "  Intelligence:".bold());
    println!("    Pending proposals: {pending_proposals}");
    println!("    Active goals:     {} ({at_risk} at risk)", goals.len());
    println!("    A/B experiments:  {running_experiments} running");
    println!();
    println!("{}", "  Revenue (30d):".bold());
    println!("    Total:            ${:.2}", revenue.total_usd);
    if !revenue.by_source.is_empty() {
        for sr in &revenue.by_source {
            println!("    {:<17} ${:.2}", format!("{}:", sr.source), sr.total_usd);
        }
    }
    println!();

    // Data sources availability.
    println!("{}", "  Data Sources:".bold());
    let sources_status = [
        ("WordPress Stats", true), // Always available via REST
        ("Search Console", true),
        ("AdSense", true),
        ("Analytics 4", true),
    ];
    for (name, available) in &sources_status {
        let status = if *available {
            "available".green().to_string()
        } else {
            "not configured".dimmed().to_string()
        };
        println!("    {:<20} {status}", name);
    }

    // Pipeline strategies.
    println!();
    println!("{}", "  Registered Strategies:".bold());
    let strategies = [
        "seo_blog",
        "metrics_collector",
        "strategy_analyzer",
        "ab_tester",
        "prompt_optimizer",
    ];
    for s in &strategies {
        println!("    - {s}");
    }

    Ok(())
}

/// Find a goal by partial ID match.
async fn find_goal(db: &DaemonDb, partial_id: &str) -> Result<jarvis_daemon_common::DaemonGoal> {
    let all = db.list_goals(&GoalFilter::default()).await?;
    all.into_iter()
        .find(|g| g.id.starts_with(partial_id))
        .ok_or_else(|| anyhow::anyhow!("no goal found matching '{partial_id}'"))
}

/// Render a simple ASCII progress bar.
fn progress_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).min(width as f64).max(0.0) as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn format_timestamp(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ts.to_string())
}
