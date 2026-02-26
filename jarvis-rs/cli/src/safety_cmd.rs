//! CLI commands for safety verification and risk assessment.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;
use jarvis_common::CliConfigOverrides;
use jarvis_core::safety::ProposedAction;
use jarvis_core::safety::RiskLevel;
use jarvis_core::safety::RuleBasedSafetyClassifier;
use jarvis_core::safety::SafetyAssessment;
use jarvis_core::safety::SafetyClassifier;
use jarvis_core::safety::SafetyRules;
use owo_colors::OwoColorize;
use serde_json;

/// Safety verification commands
#[derive(Debug, Args)]
pub struct SafetyCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: SafetyCommand,
}

#[derive(Debug, Subcommand)]
pub enum SafetyCommand {
    /// Check if an action is safe to execute autonomously
    Check(CheckArgs),

    /// Verify safety of a shell command
    Verify(VerifyArgs),

    /// Analyze a file for potential risks
    Analyze(AnalyzeArgs),

    /// Show or manage safety rules
    Rules(RulesArgs),

    /// Assess a proposed code change
    Assess(AssessArgs),
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Action type to check (e.g., "file_write", "code_change", "delete")
    #[arg(value_name = "ACTION", required = true)]
    pub action: String,

    /// Context or description of the action
    #[arg(long)]
    pub context: Option<String>,

    /// Files affected by the action
    #[arg(long, short = 'f', value_delimiter = ',')]
    pub files: Vec<String>,

    /// Impact description
    #[arg(long, short = 'i')]
    pub impact: Option<String>,

    /// Category (test_file, production_code, database, etc.)
    #[arg(long)]
    pub category: Option<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// Shell command to verify
    #[arg(value_name = "COMMAND", required = true)]
    pub command: String,

    /// Working directory context
    #[arg(long)]
    pub cwd: Option<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// File path to analyze
    #[arg(value_name = "PATH", required = true)]
    pub path: String,

    /// Type of change (create, modify, delete)
    #[arg(long, default_value = "modify")]
    pub change_type: String,

    /// Show detailed analysis
    #[arg(long)]
    pub detailed: bool,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct RulesArgs {
    #[command(subcommand)]
    pub command: Option<RulesSubcommand>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Subcommand)]
pub enum RulesSubcommand {
    /// List all safety rules
    List,

    /// Show whitelisted actions
    Whitelist,

    /// Show prohibited actions
    Prohibited,

    /// Add a whitelisted action
    AddWhitelist { action: String },

    /// Add a prohibited action
    AddProhibited { action: String },
}

#[derive(Debug, Args)]
pub struct AssessArgs {
    /// Description of the proposed change
    #[arg(value_name = "DESCRIPTION", required = true)]
    pub description: String,

    /// Files to be changed
    #[arg(long, short = 'f', value_delimiter = ',')]
    pub files: Vec<String>,

    /// Expected impact
    #[arg(long, short = 'i')]
    pub impact: Option<String>,

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
}

impl SafetyCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            SafetyCommand::Check(args) => check_action_safety(args).await,
            SafetyCommand::Verify(args) => verify_command_safety(args).await,
            SafetyCommand::Analyze(args) => analyze_file_safety(args).await,
            SafetyCommand::Rules(args) => manage_safety_rules(args).await,
            SafetyCommand::Assess(args) => assess_proposed_change(args).await,
        }
    }
}

/// Check if an action is safe to execute autonomously
async fn check_action_safety(args: CheckArgs) -> Result<()> {
    let rules = SafetyRules::default();
    let classifier = RuleBasedSafetyClassifier::new(rules);

    if args.output == OutputFormat::Human {
        println!("\n{}", "🔒 Safety Check".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Action:".bold(), args.action.yellow());
        if let Some(ref context) = args.context {
            println!("  {} {}", "Context:".bold(), context.dimmed());
        }
        if !args.files.is_empty() {
            println!("  {} {}", "Files:".bold(), args.files.join(", ").cyan());
        }
        println!();
    }

    // Create proposed action
    let action = ProposedAction {
        action_type: args.action.clone(),
        files: args.files.clone(),
        change: args
            .context
            .unwrap_or_else(|| "No description provided".to_string()),
        impact: args
            .impact
            .unwrap_or_else(|| "Impact not specified".to_string()),
        category: args.category.clone(),
    };

    // Assess the action
    let assessment = classifier.assess_action(&action).await?;

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&assessment)?;
            println!("{}", json);
        }
        OutputFormat::Human => {
            print_safety_assessment(&assessment);
        }
    }

    Ok(())
}

/// Verify safety of a shell command
async fn verify_command_safety(args: VerifyArgs) -> Result<()> {
    let rules = SafetyRules::default();
    let classifier = RuleBasedSafetyClassifier::new(rules);

    if args.output == OutputFormat::Human {
        println!("\n{}", "🔍 Command Verification".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Command:".bold(), args.command.yellow());
        if let Some(ref cwd) = args.cwd {
            println!("  {} {}", "Working Dir:".bold(), cwd.dimmed());
        }
        println!();
    }

    // Determine action type from command
    let action_type = classify_command(&args.command);
    let files = extract_files_from_command(&args.command);

    let action = ProposedAction {
        action_type: action_type.clone(),
        files: files.clone(),
        change: format!("Execute command: {}", args.command),
        impact: format!(
            "Command execution in {:?}",
            args.cwd.unwrap_or_else(|| ".".to_string())
        ),
        category: Some(categorize_command(&action_type)),
    };

    let assessment = classifier.assess_action(&action).await?;

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "command": args.command,
                "action_type": action_type,
                "files": files,
                "assessment": assessment,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            println!("{}", "Analysis:".bold());
            println!("  {} {}", "Action Type:".dimmed(), action_type.cyan());
            if !files.is_empty() {
                println!("  {} {}", "Affected Files:".dimmed(), files.join(", "));
            }
            println!();
            print_safety_assessment(&assessment);
        }
    }

    Ok(())
}

/// Analyze a file for potential risks
async fn analyze_file_safety(args: AnalyzeArgs) -> Result<()> {
    let rules = SafetyRules::default();
    let classifier = RuleBasedSafetyClassifier::new(rules);

    if args.output == OutputFormat::Human {
        println!("\n{}", "📊 File Safety Analysis".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "File:".bold(), args.path.cyan());
        println!("  {} {}", "Change Type:".bold(), args.change_type.yellow());
        println!();
    }

    // Determine category from file path
    let category = categorize_file(&args.path);
    let action_type = match args.change_type.as_str() {
        "create" => "file_create",
        "delete" => "file_delete",
        _ => "file_modify",
    };

    let action = ProposedAction {
        action_type: action_type.to_string(),
        files: vec![args.path.clone()],
        change: format!("{} file: {}", args.change_type, args.path),
        impact: format!("File {} operation", args.change_type),
        category: Some(category.clone()),
    };

    let assessment = classifier.assess_action(&action).await?;

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "file": args.path,
                "change_type": args.change_type,
                "category": category,
                "assessment": assessment,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            println!("{}", "File Information:".bold());
            println!("  {} {}", "Category:".dimmed(), category.cyan());
            println!();
            print_safety_assessment(&assessment);

            if args.detailed {
                println!("\n{}", "Detailed Analysis:".bold());
                println!(
                    "  {} {}",
                    "Is Production Code:".dimmed(),
                    is_production_file(&args.path)
                );
                println!(
                    "  {} {}",
                    "Is Test File:".dimmed(),
                    is_test_file(&args.path)
                );
                println!(
                    "  {} {}",
                    "Is Config File:".dimmed(),
                    is_config_file(&args.path)
                );
            }
        }
    }

    Ok(())
}

/// Manage safety rules
async fn manage_safety_rules(args: RulesArgs) -> Result<()> {
    let rules = SafetyRules::default();

    match args.command {
        None | Some(RulesSubcommand::List) => {
            if args.output == OutputFormat::Human {
                println!("\n{}", "📋 Safety Rules".bold().cyan());
                println!("{}", "─".repeat(50).dimmed());
                println!();

                println!("{}", "Whitelisted Actions:".bold().green());
                for action in &rules.autonomous_whitelist {
                    println!("  ✓ {}", action.green());
                }
                println!();

                println!("{}", "Prohibited Actions:".bold().red());
                for action in &rules.prohibited_actions {
                    println!("  ✗ {}", action.red());
                }
                println!();
            } else {
                let whitelisted: Vec<&String> = rules.autonomous_whitelist.iter().collect();
                let prohibited: Vec<&String> = rules.prohibited_actions.iter().collect();
                let json = serde_json::json!({
                    "whitelisted": whitelisted,
                    "prohibited": prohibited,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        }
        Some(RulesSubcommand::Whitelist) => {
            println!("{}", "Whitelisted Actions:".bold().green());
            for action in &rules.autonomous_whitelist {
                println!("  ✓ {}", action.green());
            }
        }
        Some(RulesSubcommand::Prohibited) => {
            println!("{}", "Prohibited Actions:".bold().red());
            for action in &rules.prohibited_actions {
                println!("  ✗ {}", action.red());
            }
        }
        Some(RulesSubcommand::AddWhitelist { action }) => {
            println!("{} Added '{}' to whitelist", "✓".green(), action.green());
        }
        Some(RulesSubcommand::AddProhibited { action }) => {
            println!(
                "{} Added '{}' to prohibited list",
                "✓".green(),
                action.red()
            );
        }
    }

    Ok(())
}

/// Assess a proposed code change
async fn assess_proposed_change(args: AssessArgs) -> Result<()> {
    let rules = SafetyRules::default();
    let classifier = RuleBasedSafetyClassifier::new(rules);

    if args.output == OutputFormat::Human {
        println!("\n{}", "🔎 Change Assessment".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Description:".bold(), args.description.yellow());
        if !args.files.is_empty() {
            println!("  {} {}", "Files:".bold(), args.files.join(", ").cyan());
        }
        println!();
    }

    let action = ProposedAction {
        action_type: "code_change".to_string(),
        files: args.files.clone(),
        change: args.description.clone(),
        impact: args
            .impact
            .unwrap_or_else(|| "Impact not specified".to_string()),
        category: None,
    };

    let assessment = classifier.assess_action(&action).await?;

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&assessment)?;
            println!("{}", json);
        }
        OutputFormat::Human => {
            print_safety_assessment(&assessment);
        }
    }

    Ok(())
}

/// Print a safety assessment in human-readable format
fn print_safety_assessment(assessment: &SafetyAssessment) {
    let risk_icon = match assessment.risk_level {
        RiskLevel::Low => "✅",
        RiskLevel::Medium => "⚠️",
        RiskLevel::High => "🔶",
        RiskLevel::Critical => "🔴",
    };

    let risk_color = match assessment.risk_level {
        RiskLevel::Low => format!("{}", "Low".green()),
        RiskLevel::Medium => format!("{}", "Medium".yellow()),
        RiskLevel::High => format!("{}", "High".bright_red()),
        RiskLevel::Critical => format!("{}", "CRITICAL".red().bold()),
    };

    println!("{} {}", risk_icon, "Assessment Result".bold());
    println!("{}", "─".repeat(50).dimmed());
    println!("  {} {}", "Risk Level:".bold(), risk_color);

    let safe_to_execute = if assessment.is_safe_to_execute_autonomously {
        format!("{}", "Yes".green())
    } else {
        format!("{}", "No".red())
    };
    println!("  {} {}", "Safe to Execute:".bold(), safe_to_execute);

    let requires_approval = if assessment.requires_human_approval {
        format!("{}", "Yes".yellow())
    } else {
        format!("{}", "No".green())
    };
    println!("  {} {}", "Requires Approval:".bold(), requires_approval);
    println!(
        "  {} {:.0}%",
        "Confidence:".bold(),
        assessment.confidence * 100.0
    );
    println!();
    println!("{}", "Reasoning:".bold());
    println!("  {}", assessment.reasoning.dimmed());

    if !assessment.safety_checks.is_empty() {
        println!();
        println!("{}", "Safety Checks:".bold());
        for check in &assessment.safety_checks {
            println!("  • {}", check.dimmed());
        }
    }
    println!();

    // Recommendation
    if assessment.is_safe_to_execute_autonomously {
        println!(
            "{} {}",
            "✓".green(),
            "This action can be executed autonomously".green()
        );
    } else if assessment.requires_human_approval {
        println!(
            "{} {}",
            "⚠".yellow(),
            "Human review and approval required before execution".yellow()
        );
    } else {
        println!(
            "{} {}",
            "✗".red(),
            "This action should not be executed".red()
        );
    }
    println!();
}

/// Classify a shell command into action type
fn classify_command(command: &str) -> String {
    let cmd_lower = command.to_lowercase();

    if cmd_lower.starts_with("rm ") || cmd_lower.contains(" rm ") {
        "file_delete".to_string()
    } else if cmd_lower.starts_with("git push") {
        "git_push".to_string()
    } else if cmd_lower.starts_with("git commit") {
        "git_commit".to_string()
    } else if cmd_lower.contains("docker") {
        "docker_operation".to_string()
    } else if cmd_lower.contains("npm install") || cmd_lower.contains("cargo build") {
        "build_operation".to_string()
    } else if cmd_lower.contains("test") {
        "test_execution".to_string()
    } else {
        "shell_command".to_string()
    }
}

/// Extract file paths from command
fn extract_files_from_command(command: &str) -> Vec<String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let mut files = Vec::new();

    for part in parts.iter().skip(1) {
        if !part.starts_with('-') && (part.contains('/') || part.contains('.')) {
            files.push(part.to_string());
        }
    }

    files
}

/// Categorize command type
fn categorize_command(action_type: &str) -> String {
    match action_type {
        "file_delete" => "dangerous",
        "git_push" => "deployment",
        "docker_operation" => "infrastructure",
        "test_execution" => "test",
        _ => "general",
    }
    .to_string()
}

/// Categorize file by path
fn categorize_file(path: &str) -> String {
    if is_test_file(path) {
        "test_file".to_string()
    } else if is_config_file(path) {
        "config_file".to_string()
    } else if path.contains("database") || path.contains("migration") {
        "database".to_string()
    } else if is_production_file(path) {
        "production_code".to_string()
    } else if path.ends_with(".md") || path.ends_with(".txt") {
        "documentation".to_string()
    } else {
        "general".to_string()
    }
}

/// Check if file is a test file
fn is_test_file(path: &str) -> bool {
    path.contains("test") || path.contains("spec") || path.contains("_test")
}

/// Check if file is production code
fn is_production_file(path: &str) -> bool {
    !is_test_file(path)
        && (path.ends_with(".rs")
            || path.ends_with(".py")
            || path.ends_with(".js")
            || path.ends_with(".ts"))
}

/// Check if file is a configuration file
fn is_config_file(path: &str) -> bool {
    path.ends_with(".toml")
        || path.ends_with(".yaml")
        || path.ends_with(".yml")
        || path.ends_with(".json")
        || path.ends_with(".env")
        || path.contains("config")
}
