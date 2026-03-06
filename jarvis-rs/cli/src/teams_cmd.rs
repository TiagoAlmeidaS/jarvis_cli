//! CLI commands for agent teams management and orchestration.

use anyhow::Result;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use jarvis_common::CliConfigOverrides;
use jarvis_core::config::find_jarvis_home;
use jarvis_core::teams::TeamsConfig;
use jarvis_core::teams::build_lead_prompt;
use jarvis_core::teams::load_teams;
use jarvis_core::teams::validate_teams;
use owo_colors::OwoColorize;
use std::path::PathBuf;

/// Agent teams management and orchestration commands
#[derive(Debug, Args)]
pub struct TeamsCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: TeamsCommand,
}

#[derive(Debug, Subcommand)]
pub enum TeamsCommand {
    /// List all configured teams and their teammates.
    List,

    /// Validate teams.yaml configuration.
    Validate,

    /// Show detailed information about a specific team.
    Show(ShowArgs),

    /// Run a team: spawn the lead agent with team roster awareness.
    Run(RunArgs),
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Name of the team to show.
    #[arg(value_name = "TEAM_NAME", required = true)]
    pub team_name: String,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Name of the team to run.
    #[arg(value_name = "TEAM_NAME", required = true)]
    pub team_name: String,

    /// Task description / initial prompt for the lead agent.
    #[arg(value_name = "TASK", required = true)]
    pub task: String,

    /// Working directory for the session.
    #[arg(long = "cd", short = 'C')]
    pub cwd: Option<PathBuf>,

    /// Run in full-auto mode (no approval prompts).
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,
}

impl TeamsCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            TeamsCommand::List => list_teams().await,
            TeamsCommand::Validate => validate_teams_cmd().await,
            TeamsCommand::Show(args) => show_team(args).await,
            TeamsCommand::Run(args) => run_team(args).await,
        }
    }
}

/// Resolve the effective cwd and jarvis_home, then load teams config.
fn load_teams_config() -> Result<(TeamsConfig, Vec<PathBuf>, Vec<String>)> {
    let jarvis_home = find_jarvis_home()?;
    let cwd = std::env::current_dir()?;
    let outcome = load_teams(&jarvis_home, &cwd);
    Ok((outcome.config, outcome.loaded_from, outcome.errors))
}

/// `jarvis teams list`
async fn list_teams() -> Result<()> {
    let (config, loaded_from, errors) = load_teams_config()?;

    if !errors.is_empty() {
        for err in &errors {
            eprintln!("{} {err}", "warning:".yellow());
        }
    }

    if config.teams.is_empty() {
        println!("No teams configured.");
        if loaded_from.is_empty() {
            println!("Create a {} file to define teams.", "teams.yaml".cyan());
        }
        return Ok(());
    }

    println!(
        "{} team(s) loaded from: {}",
        config.teams.len(),
        loaded_from
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    // Sort teams alphabetically for deterministic output.
    let mut teams: Vec<_> = config.teams.iter().collect();
    teams.sort_by_key(|(name, _)| name.to_owned());

    for (name, team) in &teams {
        let desc = team.description.as_deref().unwrap_or("(no description)");
        println!("{}", name.bold());
        println!("  Lead: {}", team.lead.cyan());
        println!("  Description: {}", desc.dimmed());
        println!("  Teammates: {}", team.teammates.len());

        // List teammate names sorted.
        let mut mate_names: Vec<_> = team.teammates.keys().collect();
        mate_names.sort();
        for mate_name in &mate_names {
            let mate = &team.teammates[*mate_name];
            let is_lead = mate_name.as_str() == team.lead;
            let lead_marker = if is_lead { " (lead)" } else { "" };
            let ro_marker = if mate.read_only { " [read-only]" } else { "" };
            println!(
                "    - {}{}{}: {}",
                mate_name.cyan(),
                lead_marker.green(),
                ro_marker.yellow(),
                mate.model.dimmed()
            );
        }
        println!();
    }

    Ok(())
}

/// `jarvis teams validate`
async fn validate_teams_cmd() -> Result<()> {
    let (config, loaded_from, load_errors) = load_teams_config()?;

    if !load_errors.is_empty() {
        for err in &load_errors {
            eprintln!("{} {err}", "load error:".red());
        }
    }

    if config.teams.is_empty() && loaded_from.is_empty() {
        println!("No teams.yaml found. Nothing to validate.");
        return Ok(());
    }

    println!(
        "Loaded from: {}",
        loaded_from
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let validation_errors = validate_teams(&config, None, None);

    if validation_errors.is_empty() {
        println!(
            "{} {} team(s) validated successfully.",
            "OK".green().bold(),
            config.teams.len()
        );
    } else {
        for err in &validation_errors {
            eprintln!("{} {err}", "error:".red());
        }
        eprintln!(
            "\n{} validation error(s) found.",
            validation_errors.len().to_string().red()
        );
        std::process::exit(1);
    }

    Ok(())
}

/// `jarvis teams show <team_name>`
async fn show_team(args: ShowArgs) -> Result<()> {
    let (config, _loaded_from, errors) = load_teams_config()?;

    if !errors.is_empty() {
        for err in &errors {
            eprintln!("{} {err}", "warning:".yellow());
        }
    }

    let team = config.teams.get(&args.team_name).ok_or_else(|| {
        anyhow::anyhow!(
            "team '{}' not found (available: {})",
            args.team_name,
            if config.teams.is_empty() {
                "none".to_string()
            } else {
                let mut names: Vec<_> = config.teams.keys().collect();
                names.sort();
                names
                    .iter()
                    .map(|n| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
    })?;

    println!("{}", args.team_name.bold());
    if let Some(ref desc) = team.description {
        println!("  Description: {desc}");
    }
    println!("  Lead: {}", team.lead.cyan());
    println!("  Shared memory: {}", team.shared_memory);
    println!();

    // Sort teammates alphabetically.
    let mut mates: Vec<_> = team.teammates.iter().collect();
    mates.sort_by_key(|(name, _)| name.to_owned());

    for (mate_name, mate) in &mates {
        let is_lead = mate_name.as_str() == team.lead;
        let label = if is_lead {
            format!("{} {}", mate_name.bold().cyan(), "(lead)".green())
        } else {
            format!("{}", mate_name.bold().cyan())
        };
        println!("  {label}");
        println!("    Model: {}", mate.model);

        // Show first line of role as summary.
        let role_summary = mate
            .role
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("(no role)");
        println!("    Role: {}", role_summary.dimmed());

        if !mate.skills.is_empty() {
            println!("    Skills: {}", mate.skills.join(", "));
        }
        if mate.read_only {
            println!("    Access: {}", "read-only".yellow());
        }
        if let Some(ref effort) = mate.reasoning_effort {
            println!("    Reasoning effort: {effort}");
        }
        println!();
    }

    // Show what the lead prompt would look like.
    println!("{}", "--- Generated Lead Prompt Preview ---".dimmed());
    let prompt = build_lead_prompt(&args.team_name, team);
    // Show first 40 lines as preview.
    let lines: Vec<&str> = prompt.lines().collect();
    let preview_count = 40.min(lines.len());
    for line in &lines[..preview_count] {
        println!("  {}", line.dimmed());
    }
    if lines.len() > preview_count {
        println!(
            "  {} ({} more lines)",
            "...".dimmed(),
            lines.len() - preview_count
        );
    }

    Ok(())
}

/// `jarvis teams run <team_name> <task>`
///
/// Spawns the lead agent with:
/// - The lead's model (via `-c model=...`)
/// - The lead's system prompt + team roster (via a temp file for `model_instructions_file`)
/// - The task as the user prompt
/// - Collab feature enabled so the lead can delegate via `spawn_agent`
async fn run_team(args: RunArgs) -> Result<()> {
    let (config, _loaded_from, errors) = load_teams_config()?;

    if !errors.is_empty() {
        for err in &errors {
            eprintln!("{} {err}", "warning:".yellow());
        }
    }

    let team = config.teams.get(&args.team_name).ok_or_else(|| {
        anyhow::anyhow!(
            "team '{}' not found (available: {})",
            args.team_name,
            if config.teams.is_empty() {
                "none".to_string()
            } else {
                let mut names: Vec<_> = config.teams.keys().collect();
                names.sort();
                names
                    .iter()
                    .map(|n| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
    })?;

    // Validate the team before running.
    let validation_errors = validate_teams(
        &TeamsConfig {
            teams: [(args.team_name.clone(), team.clone())]
                .into_iter()
                .collect(),
        },
        None,
        None,
    );
    if !validation_errors.is_empty() {
        for err in &validation_errors {
            eprintln!("{} {err}", "error:".red());
        }
        anyhow::bail!("team '{}' has validation errors", args.team_name);
    }

    let lead_def = team
        .teammates
        .get(&team.lead)
        .expect("lead validated to exist");

    // Build the lead prompt with team roster.
    let lead_prompt = build_lead_prompt(&args.team_name, team);

    // Write the lead prompt to a temp file so we can pass it as model_instructions_file.
    let temp_dir = tempfile::tempdir()?;
    let instructions_path = temp_dir.path().join("lead_instructions.md");
    std::fs::write(&instructions_path, &lead_prompt)?;

    // Build the exec CLI programmatically.
    let mut raw_overrides = Vec::new();

    // Set the lead's model.
    raw_overrides.push(format!("model={}", lead_def.model));

    // Point to the temp instructions file.
    let instructions_path_str = instructions_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("temp file path is not valid UTF-8"))?;
    raw_overrides.push(format!("model_instructions_file={instructions_path_str}"));

    // If the lead has reasoning_effort, set it.
    if let Some(ref effort) = lead_def.reasoning_effort {
        raw_overrides.push(format!("model_reasoning_effort={effort}"));
    }

    // Construct the ExecCli.
    let mut exec_args = vec!["jarvis".to_string(), "exec".to_string()];

    // Add -c overrides.
    for ov in &raw_overrides {
        exec_args.push("-c".to_string());
        exec_args.push(ov.clone());
    }

    // Full-auto mode.
    if args.full_auto {
        exec_args.push("--full-auto".to_string());
    }

    // Working directory.
    if let Some(ref cwd) = args.cwd {
        exec_args.push("--cd".to_string());
        exec_args.push(
            cwd.to_str()
                .ok_or_else(|| anyhow::anyhow!("cwd path is not valid UTF-8"))?
                .to_string(),
        );
    }

    // The task prompt (must be last positional arg).
    exec_args.push("--".to_string());
    exec_args.push(args.task);

    eprintln!(
        "{} Running team '{}' with lead '{}' (model: {})",
        "-->".green().bold(),
        args.team_name.cyan(),
        team.lead.cyan(),
        lead_def.model.dimmed()
    );

    let exec_cli = jarvis_exec::Cli::try_parse_from(&exec_args)?;
    jarvis_exec::run_main(exec_cli, None).await?;

    // Keep temp_dir alive until run_main completes, then it auto-cleans.
    drop(temp_dir);

    Ok(())
}
