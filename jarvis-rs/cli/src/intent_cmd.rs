//! CLI commands for intent detection system.

use anyhow::Result;
use clap::{Args, Subcommand};
use jarvis_common::CliConfigOverrides;
use jarvis_core::intent::{IntentDetector, RuleBasedIntentDetector};
use owo_colors::OwoColorize;

/// Intent detection commands
#[derive(Debug, Args)]
pub struct IntentCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: IntentCommand,
}

#[derive(Debug, Subcommand)]
pub enum IntentCommand {
    /// Detect intent from user input
    Detect(DetectArgs),

    /// List all supported intent types
    List,

    /// Test intent detection with examples
    Test,
}

#[derive(Debug, Args)]
pub struct DetectArgs {
    /// Input text to analyze
    #[arg(value_name = "INPUT", required = true)]
    pub input: String,

    /// Minimum confidence threshold (0.0-1.0)
    #[arg(long, short = 't', default_value = "0.5")]
    pub threshold: f32,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output with colors
    Human,
    /// JSON output
    Json,
    /// Simple text output
    Simple,
}

impl IntentCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            IntentCommand::Detect(args) => detect_intent(args).await,
            IntentCommand::List => list_intent_types(),
            IntentCommand::Test => test_intent_detection().await,
        }
    }
}

/// Detect intent from user input
async fn detect_intent(args: DetectArgs) -> Result<()> {
    let detector = RuleBasedIntentDetector::new(args.threshold);
    let intent = detector.detect_intent(&args.input).await?;

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&intent)?;
            println!("{}", json);
        }
        OutputFormat::Simple => {
            println!("Intent: {:?}", intent.intent_type);
            println!("Confidence: {:.2}%", intent.confidence * 100.0);
        }
        OutputFormat::Human => {
            print_human_intent(&intent, args.threshold);
        }
    }

    Ok(())
}

/// Print intent in human-readable format with colors
fn print_human_intent(intent: &jarvis_core::intent::Intent, threshold: f32) {
    println!("\n{}", "Intent Detection Result".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());

    // Intent type
    print!("\n{}: ", "Intent Type".bold());
    match intent.intent_type {
        jarvis_core::intent::IntentType::CreateSkill => {
            println!("{}", "Create Skill".green())
        }
        jarvis_core::intent::IntentType::ExecuteSkill => {
            println!("{}", "Execute Skill".blue())
        }
        jarvis_core::intent::IntentType::ListSkills => {
            println!("{}", "List Skills".yellow())
        }
        jarvis_core::intent::IntentType::Explore => println!("{}", "Explore".magenta()),
        jarvis_core::intent::IntentType::Plan => println!("{}", "Plan".cyan()),
        jarvis_core::intent::IntentType::AskCapabilities => {
            println!("{}", "Ask Capabilities".purple())
        }
        jarvis_core::intent::IntentType::NormalChat => {
            println!("{}", "Normal Chat".dimmed())
        }
    }

    // Confidence
    print!("{}: ", "Confidence".bold());
    let confidence_pct = intent.confidence * 100.0;
    if intent.is_confident(threshold) {
        println!("{:.1}% {}", confidence_pct.green(), "✓".green());
    } else {
        println!(
            "{:.1}% {} (below threshold {:.1}%)",
            confidence_pct.yellow(),
            "!".yellow(),
            threshold * 100.0
        );
    }

    // Parameters (if any)
    let params = &intent.parameters;
    let has_params = params.skill_name.is_some()
        || params.language.is_some()
        || params.skill_type.is_some()
        || params.description.is_some()
        || params.exploration_query.is_some()
        || params.planning_target.is_some();

    if has_params {
        println!("\n{}", "Parameters Extracted:".bold());
        if let Some(name) = &params.skill_name {
            println!("  {} {}", "Skill Name:".dimmed(), name);
        }
        if let Some(lang) = &params.language {
            println!("  {} {}", "Language:".dimmed(), lang);
        }
        if let Some(stype) = &params.skill_type {
            println!("  {} {}", "Skill Type:".dimmed(), stype);
        }
        if let Some(desc) = &params.description {
            println!("  {} {}", "Description:".dimmed(), desc);
        }
        if let Some(query) = &params.exploration_query {
            println!("  {} {}", "Query:".dimmed(), query);
        }
        if let Some(target) = &params.planning_target {
            println!("  {} {}", "Target:".dimmed(), target);
        }
    }

    // Raw input
    println!("\n{}", "Original Input:".bold().dimmed());
    println!("  \"{}\"", intent.raw_input.dimmed());

    println!();
}

/// List all supported intent types
fn list_intent_types() -> Result<()> {
    println!("\n{}", "Supported Intent Types".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!();

    let intents = vec![
        (
            "CreateSkill",
            "Create a new skill",
            vec!["criar skill", "create skill", "gerar skill"],
        ),
        (
            "ExecuteSkill",
            "Execute an existing skill",
            vec!["executar skill", "run skill", "usar skill"],
        ),
        (
            "ListSkills",
            "List available skills",
            vec!["listar skills", "list skills", "show skills"],
        ),
        (
            "Explore",
            "Explore the codebase",
            vec!["explore", "explorar", "analisar código"],
        ),
        (
            "Plan",
            "Create implementation plan",
            vec!["criar plano", "create plan", "how to implement"],
        ),
        (
            "AskCapabilities",
            "Ask about system capabilities",
            vec!["what can you do", "o que você pode fazer"],
        ),
        (
            "NormalChat",
            "Normal conversation",
            vec!["hello", "help", "explain"],
        ),
    ];

    for (name, description, examples) in intents {
        println!("{}", name.green().bold());
        println!("  {}", description.dimmed());
        println!("  {}: {}", "Examples".bold(), examples.join(", ").cyan());
        println!();
    }

    Ok(())
}

/// Test intent detection with predefined examples
async fn test_intent_detection() -> Result<()> {
    println!("\n{}", "Intent Detection Test Suite".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!();

    let test_cases = vec![
        "Criar uma skill para processar arquivos CSV",
        "Executar a skill de validação de email",
        "Listar todas as skills disponíveis",
        "Explorar o código de autenticação",
        "Criar um plano para implementar API REST",
        "O que você pode fazer?",
        "Olá, como você está?",
    ];

    let detector = RuleBasedIntentDetector::new(0.5);

    for (i, input) in test_cases.iter().enumerate() {
        println!("{} {}", format!("Test {}:", i + 1).bold(), input.yellow());

        let intent = detector.detect_intent(input).await?;

        print!("  {} ", "→".cyan());
        print!("{:?}", intent.intent_type);
        print!(" ({:.1}%)", intent.confidence * 100.0);

        if intent.is_confident(0.5) {
            println!(" {}", "✓".green());
        } else {
            println!(" {}", "✗".red());
        }

        println!();
    }

    println!(
        "\n{} All tests completed!",
        "✓".green().bold()
    );

    Ok(())
}
