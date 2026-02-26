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
use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;
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
use processor::router::DailyMetrics;
use processor::router::HourlyMetrics;
use processor::router::LlmRouter;
use processor::router::ProviderMetrics;
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

    /// View LLM provider metrics and performance statistics.
    Metrics {
        /// Show metrics for a specific provider (format: provider/model).
        #[clap(long)]
        provider: Option<String>,

        /// Show only providers with success rate below this threshold (0-100).
        #[clap(long)]
        min_success_rate: Option<f64>,

        /// Show metrics in JSON format.
        #[clap(long)]
        json: bool,

        /// Show hourly metrics (last 24 hours).
        #[clap(long)]
        hourly: bool,

        /// Show daily metrics (last 30 days).
        #[clap(long)]
        daily: bool,

        /// Generate HTML dashboard.
        #[clap(long)]
        dashboard: bool,
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
    // Load .env file (if present) before anything else reads env vars.
    // Tries jarvis-rs/.env first (workspace root), then the repo root .env.
    match dotenvy::dotenv() {
        Ok(path) => eprintln!("[env] Loaded {}", path.display()),
        Err(dotenvy::Error::Io(_)) => {} // no .env file — that's fine
        Err(e) => eprintln!("[env] Warning: failed to load .env: {e}"),
    }

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
        Some(Command::Metrics {
            provider,
            min_success_rate,
            json,
            hourly,
            daily,
            dashboard,
        }) => handle_metrics(provider, min_success_rate, json, hourly, daily, dashboard).await,
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
// Metrics subcommand
// ---------------------------------------------------------------------------

async fn handle_metrics(
    provider: Option<String>,
    min_success_rate: Option<f64>,
    json: bool,
    hourly: bool,
    daily: bool,
    dashboard: bool,
) -> Result<()> {
    let router = LlmRouter::new();

    // Load metrics from disk
    router.load_metrics().await?;

    if dashboard {
        // Generate HTML dashboard
        generate_dashboard(&router).await?;
        return Ok(());
    }

    if let Some(provider_id) = provider {
        // Show metrics for specific provider
        if let Some(metrics) = router.get_provider_metrics(&provider_id).await {
            if hourly {
                let hourly_data = metrics.get_hourly_metrics(None);
                if json {
                    println!("{}", serde_json::to_string_pretty(&hourly_data)?);
                } else {
                    print_hourly_metrics(&provider_id, &hourly_data);
                }
            } else if daily {
                let daily_data = metrics.get_daily_metrics(None);
                if json {
                    println!("{}", serde_json::to_string_pretty(&daily_data)?);
                } else {
                    print_daily_metrics(&provider_id, &daily_data);
                }
            } else {
                if json {
                    println!("{}", serde_json::to_string_pretty(&metrics)?);
                } else {
                    print_provider_metrics(&provider_id, &metrics);
                }
            }
        } else {
            eprintln!("No metrics found for provider: {}", provider_id);
            std::process::exit(1);
        }
    } else {
        // Show all metrics
        let all_metrics = router.get_all_metrics().await;

        if all_metrics.is_empty() {
            println!("No metrics available yet. Run some pipelines to collect metrics.");
            return Ok(());
        }

        // Filter by success rate if specified
        let filtered_metrics: Vec<_> = if let Some(min_rate) = min_success_rate {
            all_metrics
                .into_iter()
                .filter(|(_, m)| m.success_rate() < min_rate)
                .collect()
        } else {
            all_metrics.into_iter().collect()
        };

        if json {
            println!("{}", serde_json::to_string_pretty(&filtered_metrics)?);
        } else {
            print_all_metrics(&filtered_metrics);
        }
    }

    Ok(())
}

fn print_provider_metrics(provider_id: &str, metrics: &ProviderMetrics) {
    println!("Metrics for: {}\n", provider_id);
    println!("  Total Requests:      {}", metrics.total_requests);
    println!(
        "  Successful:          {} ({:.2}%)",
        metrics.successful_requests,
        metrics.success_rate()
    );
    println!(
        "  Failed:              {} ({:.2}%)",
        metrics.failed_requests,
        metrics.failure_rate()
    );
    println!("  Avg Latency:         {:.2} ms", metrics.avg_latency_ms);
    println!("  Total Cost:          ${:.4}", metrics.total_cost_usd);

    if let Some(last_success) = metrics.last_success_at {
        let dt = chrono::DateTime::from_timestamp(last_success, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("  Last Success:        {}", dt);
    }

    if let Some(last_failure) = metrics.last_failure_at {
        let dt = chrono::DateTime::from_timestamp(last_failure, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("  Last Failure:        {}", dt);
    }
}

fn print_all_metrics(metrics: &[(String, ProviderMetrics)]) {
    if metrics.is_empty() {
        println!("No metrics match the specified criteria.");
        return;
    }

    println!("LLM Provider Metrics\n");
    println!(
        "{:<40} {:>10} {:>10} {:>10} {:>12} {:>10}",
        "Provider", "Requests", "Success %", "Failure %", "Avg Latency", "Cost"
    );
    println!("{}", "-".repeat(100));

    for (provider_id, m) in metrics {
        println!(
            "{:<40} {:>10} {:>9.2}% {:>9.2}% {:>11.2} ms {:>9.4}$",
            provider_id,
            m.total_requests,
            m.success_rate(),
            m.failure_rate(),
            m.avg_latency_ms,
            m.total_cost_usd
        );
    }
}

fn print_hourly_metrics(provider_id: &str, hourly: &[&HourlyMetrics]) {
    if hourly.is_empty() {
        println!("No hourly metrics available for provider: {}", provider_id);
        return;
    }

    println!("Hourly Metrics for: {}\n", provider_id);
    println!(
        "{:<20} {:>10} {:>10} {:>10} {:>12} {:>10}",
        "Hour", "Requests", "Success", "Failed", "Avg Latency", "Cost"
    );
    println!("{}", "-".repeat(85));

    for h in hourly {
        let success_rate = if h.requests > 0 {
            (h.successful as f64 / h.requests as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{:<20} {:>10} {:>9.1}% {:>10} {:>11.2} ms {:>9.4}$",
            h.hour, h.requests, success_rate, h.failed, h.avg_latency_ms, h.cost_usd
        );
    }
}

fn print_daily_metrics(provider_id: &str, daily: &[&DailyMetrics]) {
    if daily.is_empty() {
        println!("No daily metrics available for provider: {}", provider_id);
        return;
    }

    println!("Daily Metrics for: {}\n", provider_id);
    println!(
        "{:<15} {:>10} {:>10} {:>10} {:>12} {:>10}",
        "Day", "Requests", "Success", "Failed", "Avg Latency", "Cost"
    );
    println!("{}", "-".repeat(80));

    for d in daily {
        let success_rate = if d.requests > 0 {
            (d.successful as f64 / d.requests as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "{:<15} {:>10} {:>9.1}% {:>10} {:>11.2} ms {:>9.4}$",
            d.day, d.requests, success_rate, d.failed, d.avg_latency_ms, d.cost_usd
        );
    }
}

/// Generate HTML dashboard for metrics visualization.
async fn generate_dashboard(router: &LlmRouter) -> Result<()> {
    let all_metrics = router.get_all_metrics().await;

    if all_metrics.is_empty() {
        println!("No metrics available yet. Run some pipelines to collect metrics.");
        return Ok(());
    }

    let jarvis_home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".jarvis");
    tokio::fs::create_dir_all(&jarvis_home).await?;

    let dashboard_file = jarvis_home.join("llm_dashboard.html");

    let metrics_vec: Vec<(String, ProviderMetrics)> = all_metrics.into_iter().collect();
    let html = generate_dashboard_html(&metrics_vec);
    tokio::fs::write(&dashboard_file, html).await?;

    println!("Dashboard generated: {}", dashboard_file.display());
    println!("Open it in your browser to view metrics.");

    Ok(())
}

fn generate_dashboard_html(metrics: &[(String, ProviderMetrics)]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Jarvis LLM Metrics Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: #0d1117;
            color: #c9d1d9;
            padding: 20px;
        }
        .container { max-width: 1400px; margin: 0 auto; }
        h1 { color: #58a6ff; margin-bottom: 30px; }
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .metric-card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 20px;
        }
        .metric-card h2 {
            color: #58a6ff;
            margin-bottom: 15px;
            font-size: 18px;
            border-bottom: 1px solid #30363d;
            padding-bottom: 10px;
        }
        .metric-row {
            display: flex;
            justify-content: space-between;
            padding: 8px 0;
            border-bottom: 1px solid #21262d;
        }
        .metric-row:last-child { border-bottom: none; }
        .metric-label { color: #8b949e; }
        .metric-value {
            font-weight: 600;
            color: #c9d1d9;
        }
        .success { color: #3fb950; }
        .warning { color: #d29922; }
        .error { color: #f85149; }
        .chart-container {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 20px;
        }
        .chart-title {
            color: #58a6ff;
            margin-bottom: 15px;
            font-size: 18px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 10px;
        }
        th, td {
            padding: 10px;
            text-align: left;
            border-bottom: 1px solid #21262d;
        }
        th {
            color: #58a6ff;
            font-weight: 600;
        }
        .badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: 600;
        }
        .badge-success { background: #1a7f37; color: #fff; }
        .badge-warning { background: #9e6a03; color: #fff; }
        .badge-error { background: #da3633; color: #fff; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Jarvis LLM Metrics Dashboard</h1>
        <div class="metrics-grid">
"#,
    );

    for (provider_id, m) in metrics {
        let success_class = if m.success_rate() >= 95.0 {
            "success"
        } else if m.success_rate() >= 80.0 {
            "warning"
        } else {
            "error"
        };

        html.push_str(&format!(
            r#"
            <div class="metric-card">
                <h2>{}</h2>
                <div class="metric-row">
                    <span class="metric-label">Total Requests</span>
                    <span class="metric-value">{}</span>
                </div>
                <div class="metric-row">
                    <span class="metric-label">Success Rate</span>
                    <span class="metric-value {}">{:.2}%</span>
                </div>
                <div class="metric-row">
                    <span class="metric-label">Failure Rate</span>
                    <span class="metric-value {}">{:.2}%</span>
                </div>
                <div class="metric-row">
                    <span class="metric-label">Avg Latency</span>
                    <span class="metric-value">{:.2} ms</span>
                </div>
                <div class="metric-row">
                    <span class="metric-label">Total Cost</span>
                    <span class="metric-value">${:.4}</span>
                </div>
            </div>
"#,
            provider_id,
            m.total_requests,
            success_class,
            m.success_rate(),
            if m.failure_rate() > 20.0 {
                "error"
            } else {
                "warning"
            },
            m.failure_rate(),
            m.avg_latency_ms,
            m.total_cost_usd
        ));
    }

    html.push_str(
        r#"
        </div>
        <div class="chart-container">
            <div class="chart-title">📊 Provider Comparison</div>
            <table>
                <thead>
                    <tr>
                        <th>Provider</th>
                        <th>Requests</th>
                        <th>Success Rate</th>
                        <th>Avg Latency</th>
                        <th>Total Cost</th>
                    </tr>
                </thead>
                <tbody>
"#,
    );

    for (provider_id, m) in metrics {
        let badge_class = if m.success_rate() >= 95.0 {
            "badge-success"
        } else if m.success_rate() >= 80.0 {
            "badge-warning"
        } else {
            "badge-error"
        };

        html.push_str(&format!(
            r#"
                    <tr>
                        <td>{}</td>
                        <td>{}</td>
                        <td><span class="badge {}">{:.2}%</span></td>
                        <td>{:.2} ms</td>
                        <td>${:.4}</td>
                    </tr>
"#,
            provider_id,
            m.total_requests,
            badge_class,
            m.success_rate(),
            m.avg_latency_ms,
            m.total_cost_usd
        ));
    }

    let generated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    html.push_str(&format!(
        r#"
                </tbody>
            </table>
        </div>
        <div style="text-align: center; color: #8b949e; margin-top: 40px; padding: 20px;">
            <p>Generated at: {generated_at}</p>
            <p>Refresh this page to see updated metrics</p>
        </div>
    </div>
</body>
</html>
"#
    ));

    html
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
    use jarvis_daemon_common::CreateGoal;
    use jarvis_daemon_common::GoalFilter;
    use jarvis_daemon_common::GoalMetricType;
    use jarvis_daemon_common::GoalPeriod;

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
