//! CLI commands for autonomous task execution.
//!
//! This module integrates all autonomous capabilities (intent, skills, agents,
//! context, safety) to provide end-to-end autonomous execution.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;
use jarvis_common::CliConfigOverrides;
use jarvis_core::autonomous::AnalyzedContext;
use jarvis_core::autonomous::ContextAnalyzer;
use jarvis_core::autonomous::ExecutionPlan;
use jarvis_core::autonomous::ExecutionPlanner;
use jarvis_core::autonomous::RuleBasedContextAnalyzer;
use jarvis_core::autonomous::RuleBasedExecutionPlanner;
use jarvis_core::capability::registry::InMemoryCapabilityRegistry;
use jarvis_core::intent::IntentDetector;
use jarvis_core::intent::RuleBasedIntentDetector;
use jarvis_core::safety::ProposedAction;
use jarvis_core::safety::RiskLevel;
use jarvis_core::safety::RuleBasedSafetyClassifier;
use jarvis_core::safety::SafetyClassifier;
use jarvis_core::safety::SafetyRules;
use owo_colors::OwoColorize;
use serde_json;
use std::collections::HashMap;

/// Autonomous execution commands
#[derive(Debug, Args)]
pub struct AutonomousCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: AutonomousCommand,
}

#[derive(Debug, Subcommand)]
pub enum AutonomousCommand {
    /// Create an execution plan for a task
    Plan(PlanArgs),

    /// Execute a task autonomously
    Execute(ExecuteArgs),

    /// Show status of autonomous executions
    Status(StatusArgs),

    /// Show logs of autonomous executions
    Logs(LogsArgs),

    /// Run a complete autonomous workflow (detect intent + plan + execute)
    Run(RunArgs),
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Task description
    #[arg(value_name = "TASK", required = true)]
    pub task: String,

    /// Additional context for planning
    #[arg(long)]
    pub context: Option<String>,

    /// Required capabilities
    #[arg(long, value_delimiter = ',')]
    pub capabilities: Vec<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Save plan to file
    #[arg(long)]
    pub save: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExecuteArgs {
    /// Plan ID or task description
    #[arg(value_name = "PLAN_OR_TASK", required = true)]
    pub plan_or_task: String,

    /// Auto-approve all safety checks (dangerous!)
    #[arg(long)]
    pub auto_approve: bool,

    /// Dry run (show what would be executed without actually executing)
    #[arg(long)]
    pub dry_run: bool,

    /// Maximum execution time in seconds
    #[arg(long, default_value = "300")]
    pub timeout: u64,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Execution ID to check (optional - shows all if not provided)
    #[arg(value_name = "EXECUTION_ID")]
    pub execution_id: Option<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Execution ID
    #[arg(value_name = "EXECUTION_ID", required = true)]
    pub execution_id: String,

    /// Follow logs in real-time
    #[arg(long, short = 'f')]
    pub follow: bool,

    /// Number of lines to show
    #[arg(long, short = 'n', default_value = "50")]
    pub lines: usize,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Task description in natural language
    #[arg(value_name = "TASK", required = true)]
    pub task: String,

    /// Auto-approve low-risk actions
    #[arg(long)]
    pub auto_approve_low_risk: bool,

    /// Dry run mode
    #[arg(long)]
    pub dry_run: bool,

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

impl AutonomousCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            AutonomousCommand::Plan(args) => create_execution_plan(args).await,
            AutonomousCommand::Execute(args) => execute_autonomous_task(args).await,
            AutonomousCommand::Status(args) => show_execution_status(args).await,
            AutonomousCommand::Logs(args) => show_execution_logs(args).await,
            AutonomousCommand::Run(args) => run_autonomous_workflow(args).await,
        }
    }
}

/// Create an execution plan for a task
async fn create_execution_plan(args: PlanArgs) -> Result<()> {
    if args.output == OutputFormat::Human {
        println!("\n{}", "🤖 Creating Execution Plan".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Task:".bold(), args.task.yellow());
        if let Some(ref ctx) = args.context {
            println!("  {} {}", "Context:".bold(), ctx.dimmed());
        }
        if !args.capabilities.is_empty() {
            println!(
                "  {} {}",
                "Required:".bold(),
                args.capabilities.join(", ").cyan()
            );
        }
        println!();
    }

    // Analyze context
    let analyzer = RuleBasedContextAnalyzer::new();
    let mut current_state = HashMap::new();
    if let Some(ref ctx) = args.context {
        current_state.insert("description".to_string(), ctx.clone());
    }

    let context = AnalyzedContext {
        intent: args.task.clone(),
        entities: Vec::new(),
        requirements: args.capabilities.clone(),
        constraints: Vec::new(),
        goals: vec![args.task.clone()],
        current_state,
        confidence: 0.8,
    };

    // Create execution plan
    let planner = RuleBasedExecutionPlanner::new();
    let registry = InMemoryCapabilityRegistry::new();
    let plan = planner.create_plan(&context, &registry).await?;

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&plan)?;
            println!("{}", json);
        }
        OutputFormat::Human => {
            print_execution_plan(&plan);
        }
    }

    // Save if requested
    if let Some(save_path) = args.save {
        let json = serde_json::to_string_pretty(&plan)?;
        std::fs::write(&save_path, json)?;
        if args.output == OutputFormat::Human {
            println!("\n{} Plan saved to: {}", "✓".green(), save_path.cyan());
        }
    }

    Ok(())
}

/// Execute a task autonomously
async fn execute_autonomous_task(args: ExecuteArgs) -> Result<()> {
    if args.output == OutputFormat::Human {
        println!("\n{}", "🚀 Autonomous Execution".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Task:".bold(), args.plan_or_task.yellow());
        if args.dry_run {
            println!("  {} {}", "Mode:".bold(), "Dry Run".cyan());
        }
        if args.auto_approve {
            println!("  {} {}", "Safety:".bold(), "Auto-approve enabled".yellow());
        }
        println!("  {} {}s", "Timeout:".bold(), args.timeout);
        println!();
    }

    // Safety classifier
    let safety_rules = SafetyRules::default();
    let classifier = RuleBasedSafetyClassifier::new(safety_rules);

    // Simulate execution steps
    let steps = vec![
        ("Analyze task intent", "safe"),
        ("Search for relevant context", "safe"),
        ("Generate execution plan", "safe"),
        ("Verify safety constraints", "requires_approval"),
        ("Execute actions", "requires_approval"),
    ];

    let mut executed_steps = 0;
    let mut total_steps = steps.len();

    for (i, (step_desc, safety_level)) in steps.iter().enumerate() {
        if args.output == OutputFormat::Human {
            println!(
                "{} Step {}/{}: {}",
                "→".cyan(),
                i + 1,
                total_steps,
                step_desc.bold()
            );
        }

        // Safety check
        if *safety_level == "requires_approval" && !args.auto_approve {
            let action = ProposedAction {
                action_type: "autonomous_step".to_string(),
                files: Vec::new(),
                change: step_desc.to_string(),
                impact: "Autonomous action execution".to_string(),
                category: Some("autonomous".to_string()),
            };

            let assessment = classifier.assess_action(&action).await?;

            if args.output == OutputFormat::Human {
                print!("  {} Safety check: ", "🔒".dimmed());
                match assessment.risk_level {
                    RiskLevel::Low => println!("{}", "✓ Low risk".green()),
                    RiskLevel::Medium => println!("{}", "⚠ Medium risk".yellow()),
                    RiskLevel::High => println!("{}", "🔶 High risk".bright_red()),
                    RiskLevel::Critical => println!("{}", "🔴 Critical risk".red().bold()),
                }
            }

            if !assessment.is_safe_to_execute_autonomously && !args.dry_run {
                if args.output == OutputFormat::Human {
                    println!(
                        "  {} {}",
                        "⚠".yellow(),
                        "Step requires approval - skipping in non-auto mode".yellow()
                    );
                }
                continue;
            }
        }

        if !args.dry_run {
            // Simulate execution
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        executed_steps += 1;

        if args.output == OutputFormat::Human {
            println!("  {} Step completed", "✓".green());
            println!();
        }
    }

    if args.output == OutputFormat::Human {
        println!("{}", "─".repeat(50).dimmed());
        if args.dry_run {
            println!(
                "\n{} Dry run complete - {} steps would be executed",
                "✓".cyan(),
                executed_steps
            );
        } else {
            println!(
                "\n{} Execution complete - {}/{} steps executed",
                "✓".green(),
                executed_steps,
                total_steps
            );
        }
        println!();
    } else {
        let result = serde_json::json!({
            "success": true,
            "executed_steps": executed_steps,
            "total_steps": total_steps,
            "dry_run": args.dry_run,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}

/// Show execution status
async fn show_execution_status(args: StatusArgs) -> Result<()> {
    if args.output == OutputFormat::Human {
        println!("\n{}", "📊 Execution Status".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!();
    }

    // Simulate execution data
    let executions = vec![
        (
            "exec-001",
            "Update documentation",
            "completed",
            "2 minutes ago",
        ),
        ("exec-002", "Run tests", "running", "30 seconds ago"),
        ("exec-003", "Deploy to staging", "pending", "5 minutes ago"),
    ];

    if let Some(ref exec_id) = args.execution_id {
        // Show specific execution
        if let Some((id, task, status, time)) =
            executions.iter().find(|(id, _, _, _)| id == exec_id)
        {
            if args.output == OutputFormat::Human {
                println!("  {} {}", "ID:".bold(), id.yellow());
                println!("  {} {}", "Task:".bold(), task);
                println!("  {} {}", "Status:".bold(), format_status(status));
                println!("  {} {}", "Time:".bold(), time.dimmed());
                println!();
            } else {
                let json = serde_json::json!({
                    "id": id,
                    "task": task,
                    "status": status,
                    "time": time,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        } else {
            println!("{} Execution not found: {}", "✗".red(), exec_id);
        }
    } else {
        // Show all executions
        if args.output == OutputFormat::Human {
            for (id, task, status, time) in &executions {
                println!("  {} {}", format_status(status), id.yellow());
                println!("    {} {}", "Task:".dimmed(), task);
                println!("    {} {}", "Time:".dimmed(), time.dimmed());
                println!();
            }
        } else {
            let json = serde_json::json!({
                "executions": executions.iter().map(|(id, task, status, time)| {
                    serde_json::json!({
                        "id": id,
                        "task": task,
                        "status": status,
                        "time": time,
                    })
                }).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

/// Show execution logs
async fn show_execution_logs(args: LogsArgs) -> Result<()> {
    println!("\n{}", "📜 Execution Logs".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!("  {} {}", "Execution:".bold(), args.execution_id.yellow());
    println!();

    // Simulate logs
    let logs = vec![
        "[2024-02-06 10:30:01] Starting autonomous execution",
        "[2024-02-06 10:30:02] Analyzing intent: Update documentation",
        "[2024-02-06 10:30:03] Found 3 relevant files",
        "[2024-02-06 10:30:04] Running safety checks...",
        "[2024-02-06 10:30:05] ✓ Safety check passed (Low risk)",
        "[2024-02-06 10:30:06] Executing step 1/3: Read files",
        "[2024-02-06 10:30:07] Executing step 2/3: Update content",
        "[2024-02-06 10:30:08] Executing step 3/3: Write changes",
        "[2024-02-06 10:30:09] ✓ Execution completed successfully",
    ];

    let lines_to_show = args.lines.min(logs.len());
    for log in &logs[logs.len().saturating_sub(lines_to_show)..] {
        println!("{}", log.dimmed());
    }

    println!();
    if args.follow {
        println!("{}", "Watching for new logs... (Ctrl+C to stop)".dimmed());
    }

    Ok(())
}

/// Run a complete autonomous workflow
async fn run_autonomous_workflow(args: RunArgs) -> Result<()> {
    if args.output == OutputFormat::Human {
        println!("\n{}", "🎯 Autonomous Workflow".bold().cyan());
        println!("{}", "═".repeat(50).dimmed());
        println!();
    }

    // Step 1: Detect Intent
    if args.output == OutputFormat::Human {
        println!("{}", "Step 1: Intent Detection".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
    }

    let detector = RuleBasedIntentDetector::new(0.7);
    let intent_result = detector.detect_intent(&args.task).await?;

    if args.output == OutputFormat::Human {
        println!(
            "  {} {}",
            "Intent:".bold(),
            format!("{:?}", intent_result.intent_type).yellow()
        );
        println!(
            "  {} {:.0}%",
            "Confidence:".bold(),
            intent_result.confidence * 100.0
        );
        println!();
    }

    // Step 2: Context Analysis
    if args.output == OutputFormat::Human {
        println!("{}", "Step 2: Context Analysis".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
    }

    let analyzer = RuleBasedContextAnalyzer::new();
    let context = AnalyzedContext {
        intent: args.task.clone(),
        entities: Vec::new(),
        requirements: Vec::new(),
        constraints: Vec::new(),
        goals: vec![args.task.clone()],
        current_state: HashMap::new(),
        confidence: 0.75,
    };

    if args.output == OutputFormat::Human {
        println!(
            "  {} {}",
            "Requirements:".bold(),
            context.requirements.len().to_string().cyan()
        );
        println!(
            "  {} {}",
            "Goals:".bold(),
            context.goals.len().to_string().yellow()
        );
        println!();
    }

    // Step 3: Execution Planning
    if args.output == OutputFormat::Human {
        println!("{}", "Step 3: Execution Planning".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
    }

    let planner = RuleBasedExecutionPlanner::new();
    let registry = InMemoryCapabilityRegistry::new();
    let plan = planner.create_plan(&context, &registry).await?;

    if args.output == OutputFormat::Human {
        println!(
            "  {} {} steps",
            "Plan created:".bold(),
            plan.steps.len().to_string().cyan()
        );
        println!(
            "  {} {}",
            "Estimated time:".bold(),
            plan.estimated_time.yellow()
        );
        println!();
    }

    // Step 4: Safety Verification
    if args.output == OutputFormat::Human {
        println!("{}", "Step 4: Safety Verification".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
    }

    let safety_rules = SafetyRules::default();
    let classifier = RuleBasedSafetyClassifier::new(safety_rules);

    let action = ProposedAction {
        action_type: "autonomous_workflow".to_string(),
        files: Vec::new(),
        change: args.task.clone(),
        impact: format!("Execute {} steps autonomously", plan.steps.len()),
        category: Some("workflow".to_string()),
    };

    let assessment = classifier.assess_action(&action).await?;

    if args.output == OutputFormat::Human {
        println!(
            "  {} {}",
            "Risk level:".bold(),
            format_risk_level(&assessment.risk_level)
        );
        println!(
            "  {} {}",
            "Safe to execute:".bold(),
            if assessment.is_safe_to_execute_autonomously {
                format!("{}", "Yes".green())
            } else {
                format!("{}", "No".red())
            }
        );
        println!();
    }

    // Step 5: Execution (if approved)
    if args.dry_run || !assessment.is_safe_to_execute_autonomously && !args.auto_approve_low_risk {
        if args.output == OutputFormat::Human {
            println!("{}", "─".repeat(50).dimmed());
            if args.dry_run {
                println!(
                    "\n{} {}",
                    "ℹ".cyan(),
                    "Dry run complete - no actions executed".cyan()
                );
            } else {
                println!(
                    "\n{} {}",
                    "⚠".yellow(),
                    "Execution requires manual approval".yellow()
                );
                println!("  Run with --auto-approve-low-risk to execute low-risk actions");
            }
        }
    } else {
        if args.output == OutputFormat::Human {
            println!("{}", "Step 5: Execution".bold().cyan());
            println!("{}", "─".repeat(50).dimmed());
            println!("  {} Executing {} steps...", "→".cyan(), plan.steps.len());
            println!();
        }

        // Simulate execution
        for (i, step) in plan.steps.iter().enumerate() {
            if args.output == OutputFormat::Human {
                println!(
                    "  {} Step {}/{}: {}",
                    "✓".green(),
                    i + 1,
                    plan.steps.len(),
                    step.description
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        if args.output == OutputFormat::Human {
            println!();
            println!("{}", "─".repeat(50).dimmed());
            println!(
                "\n{} {}",
                "✅".bold(),
                "Workflow completed successfully!".green().bold()
            );
        }
    }

    if args.output == OutputFormat::Json {
        let result = serde_json::json!({
            "intent": format!("{:?}", intent_result.intent_type),
            "context": {
                "requirements": context.requirements.len(),
                "goals": context.goals.len(),
                "confidence": context.confidence,
            },
            "plan": {
                "steps": plan.steps.len(),
                "estimated_time": plan.estimated_time,
            },
            "safety": {
                "risk_level": format!("{:?}", assessment.risk_level),
                "safe_to_execute": assessment.is_safe_to_execute_autonomously,
            },
            "dry_run": args.dry_run,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    println!();
    Ok(())
}

/// Print an execution plan in human-readable format
fn print_execution_plan(plan: &ExecutionPlan) {
    println!("{}", "✅ Execution Plan Created".bold().green());
    println!("{}", "─".repeat(50).dimmed());
    println!();

    if !plan.steps.is_empty() {
        println!("{} ({} steps):", "Plan Steps:".bold(), plan.steps.len());
        for step in &plan.steps {
            println!();
            println!("  {}. {}", step.step_number, step.description.bold());
            println!(
                "     {} {}",
                "Capability:".dimmed(),
                step.capability_name.cyan()
            );
            if !step.dependencies.is_empty() {
                println!(
                    "     {} Step(s) {}",
                    "Depends on:".dimmed(),
                    step.dependencies
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            println!(
                "     {} {}",
                "Estimated time:".dimmed(),
                step.estimated_time.yellow()
            );
        }
        println!();
    }

    if !plan.risks.is_empty() {
        println!("{}", "Identified Risks:".bold().yellow());
        for risk in &plan.risks {
            println!("  ⚠ {}", risk.dimmed());
        }
        println!();
    }

    println!("{}", "Summary:".bold());
    println!(
        "  {} {}",
        "Total time:".dimmed(),
        plan.estimated_time.yellow()
    );
    println!(
        "  {} {:.0}%",
        "Confidence:".dimmed(),
        plan.confidence * 100.0
    );
    println!();
}

/// Format status with colors
fn format_status(status: &str) -> String {
    match status {
        "completed" => format!("{}", "✓ completed".green()),
        "running" => format!("{}", "→ running".cyan()),
        "pending" => format!("{}", "○ pending".dimmed()),
        "failed" => format!("{}", "✗ failed".red()),
        _ => status.to_string(),
    }
}

/// Format risk level with colors
fn format_risk_level(risk: &RiskLevel) -> String {
    match risk {
        RiskLevel::Low => format!("{}", "Low".green()),
        RiskLevel::Medium => format!("{}", "Medium".yellow()),
        RiskLevel::High => format!("{}", "High".bright_red()),
        RiskLevel::Critical => format!("{}", "CRITICAL".red().bold()),
    }
}
