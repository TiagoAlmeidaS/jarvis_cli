//! CLI commands for autonomous agent operations.

use anyhow::Result;
use clap::{Args, Subcommand};
use jarvis_common::CliConfigOverrides;
use jarvis_core::agent::{
    AgentSessionManager, ExploreAgent, InMemoryAgentSessionManager, PlanAgent,
    RuleBasedExploreAgent, RuleBasedPlanAgent, Thoroughness,
};
use owo_colors::OwoColorize;
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;

/// Agent commands for autonomous operations
#[derive(Debug, Args)]
pub struct AgentCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    /// Explore a codebase autonomously
    Explore(ExploreArgs),

    /// Create an implementation plan
    Plan(PlanArgs),

    /// Manage agent sessions
    Session(SessionArgs),
}

#[derive(Debug, Args)]
pub struct ExploreArgs {
    /// Query or path to explore
    #[arg(value_name = "QUERY", required = true)]
    pub query: String,

    /// Base directory for exploration
    #[arg(long, short = 'p', default_value = ".")]
    pub path: PathBuf,

    /// Thoroughness level
    #[arg(long, short = 't', value_enum, default_value = "medium")]
    pub thoroughness: ThoroughnessArg,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Save results to file
    #[arg(long)]
    pub save: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Requirements or task description
    #[arg(value_name = "REQUIREMENTS", required = true)]
    pub requirements: String,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Save plan to file
    #[arg(long)]
    pub save: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List all agent sessions
    List(SessionListArgs),

    /// Resume a session
    Resume(SessionResumeArgs),

    /// Show session details
    Show(SessionShowArgs),
}

#[derive(Debug, Args)]
pub struct SessionListArgs {
    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct SessionResumeArgs {
    /// Session ID to resume
    #[arg(value_name = "SESSION_ID", required = true)]
    pub session_id: String,
}

#[derive(Debug, Args)]
pub struct SessionShowArgs {
    /// Session ID to show
    #[arg(value_name = "SESSION_ID", required = true)]
    pub session_id: String,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ThoroughnessArg {
    Quick,
    Medium,
    VeryThorough,
}

impl From<ThoroughnessArg> for Thoroughness {
    fn from(arg: ThoroughnessArg) -> Self {
        match arg {
            ThoroughnessArg::Quick => Thoroughness::Quick,
            ThoroughnessArg::Medium => Thoroughness::Medium,
            ThoroughnessArg::VeryThorough => Thoroughness::VeryThorough,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output with colors
    Human,
    /// JSON output
    Json,
    /// Markdown output
    Markdown,
}

impl AgentCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            AgentCommand::Explore(args) => explore_codebase(args).await,
            AgentCommand::Plan(args) => create_plan(args).await,
            AgentCommand::Session(session_args) => match session_args.command {
                SessionCommand::List(args) => list_sessions(args).await,
                SessionCommand::Resume(args) => resume_session(args).await,
                SessionCommand::Show(args) => show_session(args).await,
            },
        }
    }
}

/// Explore a codebase
async fn explore_codebase(args: ExploreArgs) -> Result<()> {
    let session_manager: Arc<dyn AgentSessionManager> =
        Arc::new(InMemoryAgentSessionManager::new());

    let agent = RuleBasedExploreAgent::new(session_manager.clone(), args.path.clone());

    if args.output == OutputFormat::Human {
        println!("\n{}", "🔍 Exploring codebase...".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Query:".bold(), args.query.yellow());
        println!("  {} {}", "Path:".bold(), args.path.display().to_string().cyan());
        println!(
            "  {} {:?}",
            "Thoroughness:".bold(),
            args.thoroughness
        );
        println!();
    }

    // Create session
    let mut session = session_manager.create_session("explore").await?;

    // Run exploration
    let thoroughness: Thoroughness = args.thoroughness.into();
    let result = agent.explore(&args.query, &mut session, thoroughness).await?;

    // Output results
    match args.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&result)?;
            println!("{}", json);
        }
        OutputFormat::Markdown => {
            print_exploration_markdown(&result);
        }
        OutputFormat::Human => {
            print_exploration_human(&result);
        }
    }

    // Save if requested
    if let Some(save_path) = args.save {
        let json = serde_json::to_string_pretty(&result)?;
        std::fs::write(&save_path, json)?;
        if args.output == OutputFormat::Human {
            println!(
                "\n{} Results saved to: {}",
                "✓".green(),
                save_path.display().to_string().cyan()
            );
        }
    }

    Ok(())
}

/// Print exploration results in human-readable format
fn print_exploration_human(result: &jarvis_core::agent::ExploreAgentResult) {
    println!("{}", "✅ Exploration Complete".bold().green());
    println!("{}", "─".repeat(50).dimmed());

    // Summary
    println!("\n{}", "Summary:".bold());
    println!("  {}", result.summary.dimmed());

    // Files explored
    if !result.files_explored.is_empty() {
        println!("\n{} ({} files):", "Files Explored:".bold(), result.files_explored.len());
        for (i, file) in result.files_explored.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, file.display().to_string().cyan());
        }
        if result.files_explored.len() > 10 {
            println!(
                "  {} ({} more files)",
                "...".dimmed(),
                result.files_explored.len() - 10
            );
        }
    }

    // Findings
    if !result.findings.is_empty() {
        println!("\n{} ({} findings):", "Key Findings:".bold(), result.findings.len());
        for (i, finding) in result.findings.iter().enumerate() {
            println!("\n  {}. {} [{}]", i + 1, finding.finding_type.green().bold(),
                     format!("{:.0}% confidence", finding.confidence * 100.0).yellow());
            println!("     {}", finding.description.dimmed());
            if !finding.files.is_empty() {
                println!("     {} {}", "Files:".bold(),
                         finding.files.iter()
                             .map(|f| f.display().to_string())
                             .collect::<Vec<_>>()
                             .join(", ").cyan());
            }
        }
    }

    // Knowledge extracted
    if !result.knowledge.is_empty() {
        println!("\n{}", "Knowledge Extracted:".bold());
        for (key, value) in result.knowledge.iter().take(5) {
            println!("  {} {}", format!("{}:", key).cyan(), value.dimmed());
        }
        if result.knowledge.len() > 5 {
            println!("  {} ({} more entries)", "...".dimmed(), result.knowledge.len() - 5);
        }
    }

    println!();
}

/// Print exploration results in markdown format
fn print_exploration_markdown(result: &jarvis_core::agent::ExploreAgentResult) {
    println!("# Exploration Results\n");
    println!("## Summary\n");
    println!("{}\n", result.summary);

    if !result.files_explored.is_empty() {
        println!("## Files Explored ({} files)\n", result.files_explored.len());
        for file in result.files_explored.iter().take(20) {
            println!("- `{}`", file.display());
        }
        if result.files_explored.len() > 20 {
            println!("\n*... and {} more files*", result.files_explored.len() - 20);
        }
        println!();
    }

    if !result.findings.is_empty() {
        println!("## Key Findings\n");
        for (i, finding) in result.findings.iter().enumerate() {
            println!("### {}. {} ({:.0}% confidence)\n",
                     i + 1, finding.finding_type, finding.confidence * 100.0);
            println!("{}\n", finding.description);
            if !finding.files.is_empty() {
                println!("**Related files:**");
                for file in &finding.files {
                    println!("- `{}`", file.display());
                }
                println!();
            }
        }
    }

    if !result.knowledge.is_empty() {
        println!("## Knowledge Extracted\n");
        for (key, value) in &result.knowledge {
            println!("- **{}**: {}", key, value);
        }
        println!();
    }
}

/// Create an implementation plan
async fn create_plan(args: PlanArgs) -> Result<()> {
    let session_manager: Arc<dyn AgentSessionManager> =
        Arc::new(InMemoryAgentSessionManager::new());

    let agent = RuleBasedPlanAgent::new(session_manager.clone());

    if args.output == OutputFormat::Human {
        println!("\n{}", "📋 Creating implementation plan...".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Requirements:".bold(), args.requirements.yellow());
        println!();
    }

    // Create session
    let mut session = session_manager.create_session("plan").await?;

    // Create plan
    let result = agent.create_plan(&args.requirements, &mut session).await?;

    // Output results
    match args.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&result)?;
            println!("{}", json);
        }
        OutputFormat::Markdown => {
            println!("{}", result.plan);
        }
        OutputFormat::Human => {
            print_plan_human(&result);
        }
    }

    // Save if requested
    if let Some(save_path) = args.save {
        let content = match args.output {
            OutputFormat::Json => serde_json::to_string_pretty(&result)?,
            _ => result.plan.clone(),
        };
        std::fs::write(&save_path, content)?;
        if args.output == OutputFormat::Human {
            println!(
                "\n{} Plan saved to: {}",
                "✓".green(),
                save_path.display().to_string().cyan()
            );
        }
    }

    Ok(())
}

/// Print plan in human-readable format
fn print_plan_human(result: &jarvis_core::agent::PlanAgentResult) {
    println!("{}", "✅ Plan Created".bold().green());
    println!("{}", "─".repeat(50).dimmed());

    // Analysis
    if !result.analysis.is_empty() {
        println!("\n{}", "Analysis:".bold().cyan());
        for line in result.analysis.lines().take(10) {
            if !line.trim().is_empty() {
                println!("{}", line.dimmed());
            }
        }
    }

    // Implementation steps
    if !result.steps.is_empty() {
        println!("\n{} ({} steps):", "Implementation Steps:".bold().cyan(), result.steps.len());
        for step in &result.steps {
            println!("\n  {}. {}", step.step_number, step.description.green());
            if !step.files.is_empty() {
                println!("     {} {}", "Files:".dimmed(), step.files.join(", ").cyan());
            }
            if !step.dependencies.is_empty() {
                println!("     {} {}", "Dependencies:".dimmed(), step.dependencies.join(", "));
            }
            println!("     {} {}", "Estimated time:".dimmed(), step.estimated_time.yellow());
        }
    }

    // Trade-offs
    if !result.trade_offs.is_empty() {
        println!("\n{}", "Trade-offs:".bold().cyan());
        for (i, trade_off) in result.trade_offs.iter().enumerate() {
            println!("\n  {}. {}", i + 1, trade_off.approach.green().bold());
            println!("     {} {}", "Pros:".bold(), trade_off.pros.join(", ").dimmed());
            println!("     {} {}", "Cons:".bold(), trade_off.cons.join(", ").dimmed());
            if let Some(rec) = &trade_off.recommendation {
                println!("     {} {}", "→".cyan(), rec.yellow());
            }
        }
    }

    // Risks
    if !result.risks.is_empty() {
        println!("\n{}", "Risks:".bold().yellow());
        for risk in &result.risks {
            let level_colored = match risk.level.as_str() {
                "high" => format!("{}", risk.level.red()),
                "medium" => format!("{}", risk.level.yellow()),
                _ => format!("{}", risk.level.blue()),
            };
            println!("  {} {}", level_colored, risk.description);
            if let Some(mitigation) = &risk.mitigation {
                println!("    {} {}", "→".dimmed(), mitigation.dimmed());
            }
        }
    }

    // Critical files
    if !result.critical_files.is_empty() {
        println!("\n{}", "Critical Files:".bold());
        for file in &result.critical_files {
            println!("  • {}", file.cyan());
        }
    }

    // Estimates
    println!("\n{}", "Estimates:".bold());
    println!("  {} {}", "Total time:".dimmed(), result.estimates.total_time.yellow());
    println!("  {} {}", "Complexity:".dimmed(), result.estimates.complexity);
    println!("  {} {}", "Steps:".dimmed(), result.estimates.step_count);

    println!();
}

/// List all sessions
async fn list_sessions(_args: SessionListArgs) -> Result<()> {
    println!("\n{}", "Agent Sessions".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!("\n{}", "Note: Session listing not yet implemented.".yellow());
    println!("Sessions are currently in-memory and will be lost on exit.");
    println!();
    Ok(())
}

/// Resume a session
async fn resume_session(args: SessionResumeArgs) -> Result<()> {
    println!("\n{}", "Resuming Session".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!("  {} {}", "Session ID:".bold(), args.session_id.yellow());
    println!("\n{}", "Note: Session resume not yet fully implemented.".yellow());
    println!("Use persistent session manager for production use.");
    println!();
    Ok(())
}

/// Show session details
async fn show_session(args: SessionShowArgs) -> Result<()> {
    let session_manager: Arc<dyn AgentSessionManager> =
        Arc::new(InMemoryAgentSessionManager::new());

    match session_manager.get_session(&args.session_id).await? {
        Some(session) => {
            match args.output {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&session)?;
                    println!("{}", json);
                }
                _ => {
                    println!("\n{}", "Session Details".bold().cyan());
                    println!("{}", "─".repeat(50).dimmed());
                    println!("  {} {}", "ID:".bold(), session.session_id.yellow());
                    println!("  {} {}", "Type:".bold(), session.agent_type.cyan());
                    println!("  {} {}", "Messages:".bold(), session.history.len());
                    println!("  {} {}", "Files Read:".bold(), session.files_read.len());
                    println!("  {} {}", "Tools Used:".bold(), session.tools_used.len());

                    if !session.history.is_empty() {
                        println!("\n{}", "Recent Messages:".bold());
                        for msg in session.history.iter().rev().take(5) {
                            println!("  {} {}", format!("[{}]:", msg.role).dimmed(),
                                     msg.content.chars().take(100).collect::<String>());
                        }
                    }
                    println!();
                }
            }
        }
        None => {
            println!("{} Session not found: {}", "✗".red(), args.session_id);
        }
    }

    Ok(())
}
