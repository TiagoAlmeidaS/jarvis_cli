//! Jarvis Daemon — background automation runner.
//!
//! This binary runs scheduled pipelines that generate and publish content
//! autonomously. It shares its SQLite database with the CLI so that
//! `jarvis daemon status` can inspect state at any time.
//!
//! ## Subcommands
//!
//! - `run` (default): start the scheduler loop.
//! - `auth google`: perform Google OAuth2 authorisation flow.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod data_sources;
mod decision_engine;
mod executor;
mod notifications;
mod pipeline;
mod pipelines;
mod processor;
mod publisher;
mod runner;
mod scheduler;
mod scraper;

use jarvis_daemon_common::DaemonDb;
use pipeline::PipelineRegistry;
use runner::PipelineRunner;
use scheduler::Scheduler;

/// Jarvis Daemon — autonomous content generation engine.
#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    about = "Jarvis Daemon — background automation runner"
)]
struct Cli {
    /// Path to the daemon SQLite database.
    /// Defaults to ~/.jarvis/daemon.db
    #[clap(long, env = "JARVIS_DAEMON_DB", global = true)]
    db_path: Option<PathBuf>,

    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the daemon scheduler loop (default when no subcommand is given).
    Run {
        /// Maximum number of concurrent pipeline jobs.
        #[clap(long, default_value = "3")]
        max_concurrent: usize,

        /// Scheduler tick interval in seconds.
        #[clap(long, default_value = "60")]
        tick_interval_sec: u64,
    },

    /// Manage external service authentication.
    Auth {
        #[clap(subcommand)]
        provider: AuthProvider,
    },
}

#[derive(Debug, Subcommand)]
enum AuthProvider {
    /// Perform Google OAuth2 authorisation (for Search Console + AdSense).
    Google {
        /// Google OAuth2 Client ID.
        #[clap(long, env = "GOOGLE_CLIENT_ID")]
        client_id: String,

        /// Google OAuth2 Client Secret.
        #[clap(long, env = "GOOGLE_CLIENT_SECRET")]
        client_secret: String,

        /// OAuth scopes (comma-separated). Defaults to Search Console + AdSense read-only.
        #[clap(long)]
        scopes: Option<String>,

        /// Path to save the credentials file.
        /// Defaults to ~/.jarvis/credentials/google.json
        #[clap(long)]
        credentials_path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env().add_directive("jarvis_daemon=info".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Auth { provider }) => handle_auth(provider).await,
        Some(Command::Run {
            max_concurrent,
            tick_interval_sec,
        }) => run_daemon(&cli.db_path, max_concurrent, tick_interval_sec).await,
        // Default to `run` when no subcommand is given (backwards-compatible).
        None => run_daemon(&cli.db_path, 3, 60).await,
    }
}

// ---------------------------------------------------------------------------
// Auth subcommand
// ---------------------------------------------------------------------------

async fn handle_auth(provider: AuthProvider) -> Result<()> {
    match provider {
        AuthProvider::Google {
            client_id,
            client_secret,
            scopes,
            credentials_path,
        } => {
            let scopes = scopes
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| {
                    vec![
                        "https://www.googleapis.com/auth/webmasters.readonly".to_string(),
                        "https://www.googleapis.com/auth/adsense.readonly".to_string(),
                    ]
                });

            let config = data_sources::google_auth::GoogleOAuthConfig {
                client_id,
                client_secret,
                scopes,
            };

            let creds_path = credentials_path
                .unwrap_or_else(data_sources::google_auth::default_credentials_path);

            let url = data_sources::google_auth::authorization_url(&config);

            println!("=== Google OAuth2 Authorization ===\n");
            println!("1. Open this URL in your browser:\n");
            println!("   {url}\n");
            println!("2. Authorize the application and copy the authorization code.\n");
            println!("3. Paste the authorization code below:\n");

            let mut code = String::new();
            std::io::stdin().read_line(&mut code)?;
            let code = code.trim();

            if code.is_empty() {
                anyhow::bail!("No authorization code provided.");
            }

            println!("\nExchanging code for tokens...");
            let tokens = data_sources::google_auth::exchange_code(&config, code).await?;
            data_sources::google_auth::save_tokens(&creds_path, &tokens)?;

            println!("Tokens saved to: {}", creds_path.display());
            println!(
                "Expires at: {}",
                chrono::DateTime::from_timestamp(tokens.expires_at, 0)
                    .map(|dt| dt.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            );
            println!(
                "\nGoogle authentication complete! The daemon can now use Search Console and AdSense APIs."
            );

            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Run subcommand (original daemon logic)
// ---------------------------------------------------------------------------

async fn run_daemon(
    db_path_opt: &Option<PathBuf>,
    max_concurrent: usize,
    tick_interval_sec: u64,
) -> Result<()> {
    let db_path = db_path_opt.clone().unwrap_or_else(|| {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".jarvis").join("daemon.db")
    });

    // Ensure parent directory exists.
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    info!("Opening daemon database at {}", db_path.display());
    let db = Arc::new(DaemonDb::open(&db_path).await?);

    // Build the pipeline registry with all known pipeline implementations.
    let registry = Arc::new(PipelineRegistry::new());

    // Build the runner.
    let runner = Arc::new(PipelineRunner::new(
        db.clone(),
        registry.clone(),
        max_concurrent,
    ));

    // Build the scheduler.
    let shutdown = CancellationToken::new();
    let scheduler = Scheduler::new(
        db.clone(),
        runner.clone(),
        std::time::Duration::from_secs(tick_interval_sec),
        shutdown.clone(),
    );

    // Listen for Ctrl+C to trigger graceful shutdown.
    let shutdown_signal = shutdown.clone();
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("Failed to listen for ctrl+c: {e}");
            return;
        }
        info!("Received Ctrl+C, initiating graceful shutdown...");
        shutdown_signal.cancel();
    });

    // Bootstrap default goals if none exist.
    if let Err(e) = bootstrap_default_goals(&db).await {
        error!("Failed to bootstrap goals: {e:#}");
    }

    // Spawn Telegram notifier if configured.
    let notifier_db = db.clone();
    let notifier_shutdown = shutdown.clone();
    tokio::spawn(async move {
        if let Err(e) = notifications::run_daily_notifier(notifier_db, notifier_shutdown).await {
            error!("Telegram notifier error: {e:#}");
        }
    });

    info!("Jarvis Daemon started (max_concurrent={max_concurrent}, tick={tick_interval_sec}s)");

    // Run the scheduler (blocks until shutdown).
    if let Err(e) = scheduler.run().await {
        error!("Scheduler error: {e:#}");
    }

    info!("Jarvis Daemon stopped.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Goal Bootstrap
// ---------------------------------------------------------------------------

/// Create default goals if the database has none.
///
/// This runs once on daemon startup. If the user has already created goals
/// (manually via CLI or from a previous run), this is a no-op.
async fn bootstrap_default_goals(db: &DaemonDb) -> Result<()> {
    use jarvis_daemon_common::{CreateGoal, GoalFilter, GoalMetricType, GoalPeriod};

    let existing = db.list_goals(&GoalFilter::default()).await?;
    if !existing.is_empty() {
        info!("{} goals already exist, skipping bootstrap", existing.len());
        return Ok(());
    }

    info!("No goals found — bootstrapping defaults...");

    let defaults = vec![
        CreateGoal {
            name: "Revenue Mensal".to_string(),
            description: Some("Receita mensal via AdSense + afiliados".to_string()),
            metric_type: GoalMetricType::Revenue,
            target_value: 200.0,
            target_unit: Some("USD".to_string()),
            period: GoalPeriod::Monthly,
            pipeline_id: None,
            priority: Some(1),
            deadline: None,
        },
        CreateGoal {
            name: "Artigos Publicados".to_string(),
            description: Some("Total de artigos publicados por mes".to_string()),
            metric_type: GoalMetricType::ContentCount,
            target_value: 90.0,
            target_unit: Some("count".to_string()),
            period: GoalPeriod::Monthly,
            pipeline_id: None,
            priority: Some(1),
            deadline: None,
        },
        CreateGoal {
            name: "Custo LLM Maximo".to_string(),
            description: Some("Manter custo LLM abaixo do limite".to_string()),
            metric_type: GoalMetricType::CostLimit,
            target_value: 5.0,
            target_unit: Some("USD".to_string()),
            period: GoalPeriod::Monthly,
            pipeline_id: None,
            priority: Some(2),
            deadline: None,
        },
        CreateGoal {
            name: "Pageviews Mensais".to_string(),
            description: Some("Total de visualizacoes por mes".to_string()),
            metric_type: GoalMetricType::Pageviews,
            target_value: 10_000.0,
            target_unit: Some("count".to_string()),
            period: GoalPeriod::Monthly,
            pipeline_id: None,
            priority: Some(2),
            deadline: None,
        },
        CreateGoal {
            name: "Clicks Mensais".to_string(),
            description: Some("Total de clicks organicos por mes (Search Console)".to_string()),
            metric_type: GoalMetricType::Clicks,
            target_value: 5_000.0,
            target_unit: Some("count".to_string()),
            period: GoalPeriod::Monthly,
            pipeline_id: None,
            priority: Some(3),
            deadline: None,
        },
    ];

    for goal in &defaults {
        match db.create_goal(goal).await {
            Ok(created) => info!("Goal created: {} (P{})", created.name, created.priority),
            Err(e) => error!("Failed to create goal '{}': {e}", goal.name),
        }
    }

    info!("{} default goals bootstrapped", defaults.len());
    Ok(())
}
