//! CLI commands for skill development and management.

use anyhow::Result;
use clap::{Args, Subcommand};
use jarvis_common::CliConfigOverrides;
use jarvis_core::intent::IntentParameters;
use jarvis_core::skills::{
    LLMSkillDevelopmentService, RuleBasedSkillEvaluator, SkillDefinition, SkillDevelopmentService,
    SkillEvaluator,
};
use owo_colors::OwoColorize;
use serde_json;

/// Skill development and management commands
#[derive(Debug, Args)]
pub struct SkillsCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: SkillsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SkillsCommand {
    /// Create a new skill from requirements
    Create(CreateArgs),

    /// Evaluate skill quality
    Evaluate(EvaluateArgs),

    /// List available skills
    List(ListArgs),

    /// Test skill generation with examples
    Test,
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Name for the skill
    #[arg(value_name = "NAME", required = true)]
    pub name: String,

    /// Skill requirements or description
    #[arg(value_name = "REQUIREMENTS", required = true)]
    pub requirements: String,

    /// Programming language (rust, python, javascript)
    #[arg(long, short = 'l', default_value = "rust")]
    pub language: String,

    /// Skill type (api, library, component, script, console)
    #[arg(long, short = 't', default_value = "library")]
    pub skill_type: String,

    /// Auto-evaluate after generation
    #[arg(long, default_value = "true")]
    pub evaluate: bool,

    /// Minimum quality threshold for evaluation
    #[arg(long, default_value = "0.5")]
    pub quality_threshold: f32,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct EvaluateArgs {
    /// Path to skill definition JSON file
    #[arg(value_name = "SKILL_FILE", required = true)]
    pub skill_file: std::path::PathBuf,

    /// Minimum quality threshold
    #[arg(long, short = 't', default_value = "0.5")]
    pub threshold: f32,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct ListArgs {
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

impl SkillsCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            SkillsCommand::Create(args) => create_skill(args).await,
            SkillsCommand::Evaluate(args) => evaluate_skill(args).await,
            SkillsCommand::List(args) => list_skills(args).await,
            SkillsCommand::Test => test_skill_generation().await,
        }
    }
}

/// Create a new skill from requirements
async fn create_skill(args: CreateArgs) -> Result<()> {
    let service = LLMSkillDevelopmentService::new(args.language.clone(), args.skill_type.clone());

    // Build parameters from args
    let mut parameters = IntentParameters::default();
    parameters.skill_name = Some(args.name.clone());
    parameters.language = Some(args.language.clone());
    parameters.skill_type = Some(args.skill_type.clone());
    parameters.description = Some(args.requirements.clone());

    // Generate skill
    if args.output == OutputFormat::Human {
        println!("\n{}", "🔨 Generating skill...".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
    }

    let result = service
        .generate_skill(&args.requirements, &parameters)
        .await?;

    // Evaluate if requested
    let evaluation_result = if args.evaluate {
        if args.output == OutputFormat::Human {
            println!("\n{}", "📊 Evaluating skill quality...".bold().cyan());
        }

        let evaluator = RuleBasedSkillEvaluator::new(args.quality_threshold);
        Some(evaluator.evaluate_skill(&result.skill).await?)
    } else {
        None
    };

    // Output results
    match args.output {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "skill": result.skill,
                "success": result.success,
                "errors": result.errors,
                "warnings": result.warnings,
                "evaluation": evaluation_result,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Simple => {
            println!("Skill: {}", result.skill.name);
            println!("Language: {}", result.skill.language);
            println!("Type: {}", result.skill.skill_type);
            println!("Success: {}", result.success);
            if let Some(eval) = evaluation_result {
                println!("Quality: {:.1}%", eval.metrics.quality_score * 100.0);
                println!("Passed: {}", eval.passed);
            }
        }
        OutputFormat::Human => {
            print_skill_result(&result, evaluation_result.as_ref());
        }
    }

    Ok(())
}

/// Print skill generation result in human-readable format
fn print_skill_result(
    result: &jarvis_core::skills::SkillDevelopmentResult,
    evaluation: Option<&jarvis_core::skills::SkillEvaluationResult>,
) {
    println!("\n{}", "✅ Skill Generation Complete".bold().green());
    println!("{}", "─".repeat(50).dimmed());

    let skill = &result.skill;

    // Basic info
    println!("\n{}", "Skill Information:".bold());
    println!("  {} {}", "Name:".dimmed(), skill.name.cyan());
    println!("  {} {}", "Language:".dimmed(), skill.language.yellow());
    println!("  {} {}", "Type:".dimmed(), skill.skill_type.yellow());
    println!("  {} {}", "Version:".dimmed(), skill.version);

    // Description
    if !skill.description.is_empty() {
        println!("\n{}", "Description:".bold());
        println!("  {}", skill.description.dimmed());
    }

    // Dependencies
    if !skill.dependencies.is_empty() {
        println!("\n{}", "Dependencies:".bold());
        for dep in &skill.dependencies {
            println!("  - {}", dep.cyan());
        }
    }

    // Code preview
    println!("\n{}", "Generated Code:".bold());
    let lines: Vec<&str> = skill.code.lines().collect();
    let preview_lines = lines.iter().take(20);
    for (i, line) in preview_lines.enumerate() {
        println!("  {:3} | {}", i + 1, line.dimmed());
    }
    if lines.len() > 20 {
        println!("  {} ({} more lines)", "...".dimmed(), lines.len() - 20);
    }

    // Test code if available
    if let Some(test_code) = &skill.test_code {
        if !test_code.is_empty() {
            println!("\n{}", "Test Code:".bold());
            let test_lines: Vec<&str> = test_code.lines().collect();
            let preview_lines = test_lines.iter().take(10);
            for (i, line) in preview_lines.enumerate() {
                println!("  {:3} | {}", i + 1, line.dimmed());
            }
            if test_lines.len() > 10 {
                println!(
                    "  {} ({} more lines)",
                    "...".dimmed(),
                    test_lines.len() - 10
                );
            }
        }
    }

    // Evaluation results
    if let Some(eval) = evaluation {
        println!("\n{}", "Quality Evaluation:".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());

        let metrics = &eval.metrics;

        // Overall quality
        print!("\n{}: ", "Overall Quality".bold());
        let quality_pct = metrics.quality_score * 100.0;
        if eval.passed {
            println!("{:.1}% {}", quality_pct.green(), "✓".green());
        } else {
            println!("{:.1}% {} (below threshold)", quality_pct.red(), "✗".red());
        }

        // Other metrics
        println!("\n{}", "Detailed Metrics:".bold());
        println!(
            "  {} {:.1}%",
            "Complexity:".dimmed(),
            metrics.complexity_score * 100.0
        );
        println!(
            "  {} {:.1}%",
            "Maintainability:".dimmed(),
            metrics.maintainability_score * 100.0
        );
        println!(
            "  {} {:.1}%",
            "Test Coverage:".dimmed(),
            metrics.test_coverage * 100.0
        );
        println!("  {} {}", "Lines of Code:".dimmed(), metrics.lines_of_code);
        println!("  {} {}", "Functions:".dimmed(), metrics.function_count);

        // Issues
        if !metrics.issues.is_empty() {
            println!("\n{}", "Issues Found:".bold().yellow());
            for issue in &metrics.issues {
                let severity_display = match issue.severity.as_str() {
                    "high" => format!("{}", issue.severity.red()),
                    "medium" => format!("{}", issue.severity.yellow()),
                    _ => format!("{}", issue.severity.blue()),
                };
                println!("  {} {}", severity_display, issue.description);
                if let Some(suggestion) = &issue.suggestion {
                    println!("    {} {}", "→".dimmed(), suggestion.dimmed());
                }
            }
        }

        // Recommendations
        if !metrics.recommendations.is_empty() {
            println!("\n{}", "Recommendations:".bold().blue());
            for rec in &metrics.recommendations {
                println!("  • {}", rec);
            }
        }
    }

    // Errors and warnings
    if !result.errors.is_empty() {
        println!("\n{}", "Errors:".bold().red());
        for error in &result.errors {
            println!("  {} {}", "✗".red(), error);
        }
    }

    if !result.warnings.is_empty() {
        println!("\n{}", "Warnings:".bold().yellow());
        for warning in &result.warnings {
            println!("  {} {}", "⚠".yellow(), warning);
        }
    }

    println!();
}

/// Evaluate an existing skill
async fn evaluate_skill(args: EvaluateArgs) -> Result<()> {
    // Read skill definition from file
    let content = std::fs::read_to_string(&args.skill_file)?;
    let skill: SkillDefinition = serde_json::from_str(&content)?;

    if args.output == OutputFormat::Human {
        println!("\n{}", "📊 Evaluating skill...".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "File:".dimmed(), args.skill_file.display());
        println!("  {} {}", "Skill:".dimmed(), skill.name.cyan());
    }

    // Evaluate
    let evaluator = RuleBasedSkillEvaluator::new(args.threshold);
    let result = evaluator.evaluate_skill(&skill).await?;

    // Output results
    match args.output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Simple => {
            println!("Quality: {:.1}%", result.metrics.quality_score * 100.0);
            println!("Passed: {}", result.passed);
            println!("Issues: {}", result.metrics.issues.len());
        }
        OutputFormat::Human => {
            print_evaluation_result(&skill, &result);
        }
    }

    Ok(())
}

/// Print evaluation result in human-readable format
fn print_evaluation_result(
    skill: &SkillDefinition,
    result: &jarvis_core::skills::SkillEvaluationResult,
) {
    println!("\n{}", "Quality Evaluation Result".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());

    let metrics = &result.metrics;

    // Overall status
    print!("\n{}: ", "Status".bold());
    if result.passed {
        println!("{} {}", "PASSED".green().bold(), "✓".green());
    } else {
        println!("{} {}", "FAILED".red().bold(), "✗".red());
    }

    // Quality score
    print!("{}: ", "Quality Score".bold());
    let quality_pct = metrics.quality_score * 100.0;
    if result.passed {
        println!("{:.1}%", quality_pct.green());
    } else {
        println!(
            "{:.1}% {} (threshold: {:.1}%)",
            quality_pct.red(),
            "↓".red(),
            result.threshold * 100.0
        );
    }

    // Detailed metrics
    println!("\n{}", "Metrics:".bold());
    println!(
        "  {} {:.1}%",
        "Complexity:".dimmed(),
        metrics.complexity_score * 100.0
    );
    println!(
        "  {} {:.1}%",
        "Maintainability:".dimmed(),
        metrics.maintainability_score * 100.0
    );
    println!(
        "  {} {:.1}%",
        "Test Coverage:".dimmed(),
        metrics.test_coverage * 100.0
    );
    println!("  {} {}", "Lines of Code:".dimmed(), metrics.lines_of_code);
    println!("  {} {}", "Functions:".dimmed(), metrics.function_count);

    // Issues
    if !metrics.issues.is_empty() {
        println!("\n{}", "Issues:".bold().yellow());
        for (i, issue) in metrics.issues.iter().enumerate() {
            let severity_display = match issue.severity.as_str() {
                "high" => format!("{}", issue.severity.red()),
                "medium" => format!("{}", issue.severity.yellow()),
                _ => format!("{}", issue.severity.blue()),
            };
            println!("  {}. {} {}", i + 1, severity_display, issue.description);
            if let Some(suggestion) = &issue.suggestion {
                println!("     {} {}", "→".dimmed(), suggestion.dimmed());
            }
        }
    }

    // Recommendations
    if !metrics.recommendations.is_empty() {
        println!("\n{}", "Recommendations:".bold().blue());
        for (i, rec) in metrics.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, rec);
        }
    }

    println!();
}

/// List available skills
async fn list_skills(_args: ListArgs) -> Result<()> {
    println!("\n{}", "Available Skills".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!(
        "\n{}",
        "Note: Skill listing from manager not yet implemented.".yellow()
    );
    println!(
        "Use {} to explore skills directory.",
        "jarvis agent explore".cyan()
    );
    println!();
    Ok(())
}

/// Test skill generation with examples
async fn test_skill_generation() -> Result<()> {
    println!("\n{}", "Skill Generation Test Suite".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!();

    let test_cases = vec![
        (
            "csv_processor",
            "Process CSV files and extract data",
            "rust",
            "library",
        ),
        (
            "email_validator",
            "Validate email addresses using regex",
            "python",
            "library",
        ),
        (
            "rest_api_client",
            "HTTP client for REST API requests",
            "javascript",
            "api",
        ),
    ];

    for (i, (name, requirements, language, skill_type)) in test_cases.iter().enumerate() {
        println!(
            "{} {} ({}, {})",
            format!("Test {}:", i + 1).bold(),
            name.yellow(),
            language.cyan(),
            skill_type.cyan()
        );

        let service = LLMSkillDevelopmentService::new(language.to_string(), skill_type.to_string());

        let mut parameters = IntentParameters::default();
        parameters.skill_name = Some(name.to_string());
        parameters.language = Some(language.to_string());
        parameters.skill_type = Some(skill_type.to_string());
        parameters.description = Some(requirements.to_string());

        match service.generate_skill(requirements, &parameters).await {
            Ok(result) => {
                print!("  {} Generated successfully", "→".cyan());

                // Quick evaluation
                let evaluator = RuleBasedSkillEvaluator::new(0.5);
                match evaluator.evaluate_skill(&result.skill).await {
                    Ok(eval) => {
                        let quality_pct = eval.metrics.quality_score * 100.0;
                        if eval.passed {
                            println!(" - Quality: {:.1}% {}", quality_pct.green(), "✓".green());
                        } else {
                            println!(" - Quality: {:.1}% {}", quality_pct.yellow(), "⚠".yellow());
                        }
                    }
                    Err(e) => println!(" - Evaluation failed: {}", e.to_string().red()),
                }
            }
            Err(e) => {
                println!("  {} Generation failed: {}", "✗".red(), e.to_string().red());
            }
        }

        println!();
    }

    println!("{} All tests completed!", "✓".green().bold());

    Ok(())
}
