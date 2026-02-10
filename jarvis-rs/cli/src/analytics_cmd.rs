//! CLI commands for system analytics and self-improvement.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use jarvis_common::CliConfigOverrides;
use jarvis_core::analytics::{Improvement, ImprovementPriority, SelfImprovement, group_by_priority};
use jarvis_core::integrations::redis::{MultiLevelCache, RedisCache};
use jarvis_core::integrations::sqlserver::Database;
use std::sync::Arc;
use owo_colors::OwoColorize;
use serde_json;
use std::time::Duration;

/// Analytics and self-improvement commands
#[derive(Debug, Args)]
pub struct AnalyticsCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: AnalyticsCommand,
}

#[derive(Debug, Subcommand)]
pub enum AnalyticsCommand {
    /// Analyze system performance and suggest improvements
    Analyze(AnalyzeArgs),

    /// Show system metrics dashboard
    Dashboard(DashboardArgs),

    /// Show cache performance metrics
    Cache(CacheArgs),

    /// Show command execution metrics
    Commands(CommandsArgs),

    /// Show skill usage metrics
    Skills(SkillsArgs),
}

#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// Database connection string
    #[arg(long, env = "JARVIS_DB_CONNECTION_STRING")]
    pub db_connection: Option<String>,

    /// Redis connection string (optional)
    #[arg(long, env = "JARVIS_REDIS_URL")]
    pub redis_url: Option<String>,

    /// Minimum priority to show (critical, high, medium, low)
    #[arg(long, short = 'p', default_value = "low")]
    pub min_priority: String,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Only show critical improvements
    #[arg(long)]
    pub critical_only: bool,
}

#[derive(Debug, Args)]
pub struct DashboardArgs {
    /// Database connection string
    #[arg(long, env = "JARVIS_DB_CONNECTION_STRING")]
    pub db_connection: Option<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct CacheArgs {
    /// Redis connection string
    #[arg(long, env = "JARVIS_REDIS_URL")]
    pub redis_url: Option<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct CommandsArgs {
    /// Database connection string
    #[arg(long, env = "JARVIS_DB_CONNECTION_STRING")]
    pub db_connection: Option<String>,

    /// Limit number of results
    #[arg(long, short = 'l', default_value = "10")]
    pub limit: i32,

    /// Show only slow commands (> 2000ms)
    #[arg(long)]
    pub slow_only: bool,

    /// Show only unreliable commands (< 80% success rate)
    #[arg(long)]
    pub unreliable_only: bool,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct SkillsArgs {
    /// Database connection string
    #[arg(long, env = "JARVIS_DB_CONNECTION_STRING")]
    pub db_connection: Option<String>,

    /// Limit number of results
    #[arg(long, short = 'l', default_value = "10")]
    pub limit: i32,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output with colors
    Human,
    /// JSON output
    Json,
    /// Simple text output
    Simple,
}

impl AnalyticsCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            AnalyticsCommand::Analyze(args) => analyze_system(args).await,
            AnalyticsCommand::Dashboard(args) => show_dashboard(args).await,
            AnalyticsCommand::Cache(args) => show_cache_metrics(args).await,
            AnalyticsCommand::Commands(args) => show_command_metrics(args).await,
            AnalyticsCommand::Skills(args) => show_skill_metrics(args).await,
        }
    }
}

/// Analyze system performance and suggest improvements
async fn analyze_system(args: AnalyzeArgs) -> Result<()> {
    let db_connection = args.db_connection
        .context("Database connection string required (use --db-connection or JARVIS_DB_CONNECTION_STRING)")?;

    // Initialize database
    let db = Database::new(&db_connection).await
        .context("Failed to connect to database")?;

    // Initialize cache if Redis URL provided
    let cache = if let Some(redis_url) = args.redis_url {
        let redis_cache = RedisCache::new(&redis_url).await
            .context("Failed to connect to Redis")?;
        Some(MultiLevelCache::new(
            Some(Arc::new(redis_cache)),
            Duration::from_secs(300),  // L1 TTL: 5 minutes
            Duration::from_secs(3600), // L2 TTL: 1 hour
        ))
    } else {
        None
    };

    // Create self-improvement service
    let service = SelfImprovement::new(db, cache);

    // Analyze and get suggestions
    if args.output == OutputFormat::Human {
        println!("\n{}", "🔍 Analyzing Jarvis performance...".bold().cyan());
        println!("{}", "─".repeat(60).dimmed());
    }

    let improvements = service.analyze_and_suggest().await
        .context("Failed to analyze system")?;

    // Filter by minimum priority
    let filtered_improvements = if args.critical_only {
        improvements.into_iter()
            .filter(|i| i.priority == ImprovementPriority::Critical)
            .collect()
    } else {
        improvements
    };

    if filtered_improvements.is_empty() {
        if args.output == OutputFormat::Human {
            println!("\n{} {}", "✅".green(), "No improvements needed! System is running optimally.".bold().green());
        }
        return Ok(());
    }

    // Output results
    match args.output {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "improvements": filtered_improvements,
                "total": filtered_improvements.len(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            print_improvements_human(&filtered_improvements);
        }
        OutputFormat::Simple => {
            print_improvements_simple(&filtered_improvements);
        }
    }

    Ok(())
}

/// Show system metrics dashboard
async fn show_dashboard(args: DashboardArgs) -> Result<()> {
    let db_connection = args.db_connection
        .context("Database connection string required")?;

    let _db = Database::new(&db_connection).await
        .context("Failed to connect to database")?;

    match args.output {
        OutputFormat::Human => {
            println!("\n{}", "📊 Jarvis System Dashboard".bold().cyan());
            println!("{}", "─".repeat(60).dimmed());
            println!("\n{}", "⚠️  Dashboard implementation coming soon!".yellow());
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "not_implemented",
                "message": "Dashboard implementation coming soon"
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Simple => {
            println!("Dashboard implementation coming soon");
        }
    }

    Ok(())
}

/// Show cache performance metrics
async fn show_cache_metrics(args: CacheArgs) -> Result<()> {
    if args.redis_url.is_none() {
        anyhow::bail!("Redis connection string required (use --redis-url or JARVIS_REDIS_URL)");
    }

    match args.output {
        OutputFormat::Human => {
            println!("\n{}", "💾 Cache Performance Metrics".bold().cyan());
            println!("{}", "─".repeat(60).dimmed());
            println!("\n{}", "⚠️  Cache metrics implementation coming soon!".yellow());
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "not_implemented",
                "message": "Cache metrics implementation coming soon"
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Simple => {
            println!("Cache metrics implementation coming soon");
        }
    }

    Ok(())
}

/// Show command execution metrics
async fn show_command_metrics(args: CommandsArgs) -> Result<()> {
    let db_connection = args.db_connection
        .context("Database connection string required")?;

    let _db = Database::new(&db_connection).await
        .context("Failed to connect to database")?;

    match args.output {
        OutputFormat::Human => {
            println!("\n{}", "⚡ Command Execution Metrics".bold().cyan());
            println!("{}", "─".repeat(60).dimmed());
            println!("\n{}", "⚠️  Command metrics implementation coming soon!".yellow());
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "not_implemented",
                "message": "Command metrics implementation coming soon"
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Simple => {
            println!("Command metrics implementation coming soon");
        }
    }

    Ok(())
}

/// Show skill usage metrics
async fn show_skill_metrics(args: SkillsArgs) -> Result<()> {
    let db_connection = args.db_connection
        .context("Database connection string required")?;

    let _db = Database::new(&db_connection).await
        .context("Failed to connect to database")?;

    match args.output {
        OutputFormat::Human => {
            println!("\n{}", "🎯 Skill Usage Metrics".bold().cyan());
            println!("{}", "─".repeat(60).dimmed());
            println!("\n{}", "⚠️  Skill metrics implementation coming soon!".yellow());
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "not_implemented",
                "message": "Skill metrics implementation coming soon"
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Simple => {
            println!("Skill metrics implementation coming soon");
        }
    }

    Ok(())
}

/// Print improvements in human-readable format
fn print_improvements_human(improvements: &[Improvement]) {
    let grouped = group_by_priority(improvements.to_vec());

    println!("\n{} {} improvements found\n", "📋".bold(), improvements.len().to_string().bold().yellow());

    for (priority, items) in grouped {
        let count: usize = items.len();
        println!("{} {} ({} items)", "▶".bold(), priority, count.to_string().bold());
        println!("{}", "─".repeat(60).dimmed());

        for (idx, improvement) in items.iter().enumerate() {
            let num: usize = idx + 1;
            println!("\n  {}. {} {}",
                num.to_string().bold(),
                improvement.category,
                improvement.title.bold()
            );
            println!("     {}", improvement.description.dimmed());

            if let Some(action) = &improvement.action {
                println!("     {} {}", "→".cyan(), action);
            }

            if let Some(impact) = &improvement.impact {
                println!("     {} {}", "💡".cyan(), impact);
            }
        }
        println!();
    }
}

/// Print improvements in simple text format
fn print_improvements_simple(improvements: &[Improvement]) {
    for improvement in improvements {
        println!("[{}] {} - {}",
            match improvement.priority {
                ImprovementPriority::Critical => "CRITICAL",
                ImprovementPriority::High => "HIGH",
                ImprovementPriority::Medium => "MEDIUM",
                ImprovementPriority::Low => "LOW",
            },
            improvement.title,
            improvement.description
        );

        if let Some(action) = &improvement.action {
            println!("  Action: {}", action);
        }

        if let Some(impact) = &improvement.impact {
            println!("  Impact: {}", impact);
        }

        println!();
    }
}
