//! Code implementation engine using the Jarvis sub-agent.
//!
//! This module implements the execution phase of the autonomous issue resolution
//! pipeline. It uses `run_codex_thread_interactive` to spawn a sub-agent that
//! performs the actual code implementation with iterative fix→test cycles.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use anyhow::Result;
use jarvis_github::GitHubClient;
use jarvis_protocol::protocol::AskForApproval;
use jarvis_protocol::protocol::EventMsg;
use jarvis_protocol::protocol::SandboxPolicy;
use jarvis_protocol::user_input::UserInput;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use crate::AuthManager;
use crate::Session;
use crate::TurnContext;
use crate::config::Config;
use crate::config::Constrained;
use crate::jarvis_delegate::run_codex_thread_interactive;
use crate::models_manager::manager::ModelsManager;

use super::types::ExecutionResult;
use super::types::ExecutionStatus;
use super::types::ImplementationPlan;
use super::types::RepoContext;
use super::types::TestRunResult;

const MAX_ITERATIONS: u32 = 5;
const IMPLEMENTATION_SYSTEM_PROMPT: &str = r###"You are an autonomous code implementation engine. Your task is to implement the requested feature or fix based on the provided issue and implementation plan.

## Working Context
- You are working in a local clone of the repository
- All shell commands execute in the repository root
- You have full read/write access to the filesystem

## Your Workflow
1. **Understand the issue and plan** - Review the issue description and implementation steps
2. **Implement the changes** - Make code changes as described in the plan
3. **Run tests** - Execute the test commands to validate your implementation
4. **Iterate if needed** - If tests fail, analyze the errors and fix them

## Guidelines
- Follow the existing code style and patterns in the repository
- Write tests for new functionality when appropriate
- Keep changes focused on the scope of the issue
- If you cannot complete the implementation, explain what blockers exist

## Output
When complete, provide a summary of:
- Files changed
- Tests added/modified
- How the implementation addresses the issue
"###;

pub struct ImplementationExecutor;

impl ImplementationExecutor {
    pub async fn execute(
        config: &Config,
        auth_manager: Arc<AuthManager>,
        models_manager: Arc<ModelsManager>,
        parent_session: Arc<Session>,
        parent_ctx: Arc<TurnContext>,
        cancel_token: CancellationToken,
        plan: &ImplementationPlan,
        repo_context: &RepoContext,
        github_client: &GitHubClient,
        repo_clone_path: &PathBuf,
    ) -> Result<ExecutionResult> {
        info!(
            "starting implementation for branch={}, {} steps",
            plan.branch_name,
            plan.steps.len()
        );

        let branch_name = plan.branch_name.clone();
        let test_commands = plan.test_commands.clone();

        let mut sub_config = config.clone();
        sub_config.base_instructions = Some(IMPLEMENTATION_SYSTEM_PROMPT.to_string());
        sub_config.cwd = repo_clone_path.clone();
        sub_config.approval_policy = Constrained::allow_any(AskForApproval::Never);
        sub_config.sandbox_policy = Constrained::allow_any(SandboxPolicy::DangerFullAccess);

        if let Some(model) = config.model.clone() {
            sub_config.model = Some(model);
        }

        let implementation_prompt = Self::build_implementation_prompt(plan, repo_context);

        let jarvis = run_codex_thread_interactive(
            sub_config,
            auth_manager,
            models_manager,
            parent_session,
            parent_ctx,
            cancel_token.clone(),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to start implementation sub-agent: {e}"))?;

        jarvis
            .submit(Op::UserInput {
                items: vec![UserInput::Text {
                    text: implementation_prompt,
                    text_elements: vec![],
                }],
                final_output_json_schema: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!("failed to submit implementation prompt: {e}"))?;

        let mut iterations: u32 = 0;
        let mut last_agent_summary: Option<String> = None;
        let mut test_results: Vec<TestRunResult> = Vec::new();
        let mut final_status = ExecutionStatus::Success;

        loop {
            iterations += 1;
            info!("implementation iteration {}/{}", iterations, MAX_ITERATIONS);

            let mut turn_complete = false;
            let mut turn_aborted = false;

            while let Ok(event) = jarvis.next_event().await {
                match event.msg {
                    EventMsg::TurnComplete(tc) => {
                        last_agent_summary = tc.last_agent_message;
                        turn_complete = true;
                        break;
                    }
                    EventMsg::TurnAborted(_) => {
                        turn_aborted = true;
                        break;
                    }
                    _ => {}
                }
            }

            if turn_aborted {
                final_status = ExecutionStatus::AgentError;
                break;
            }

            if !turn_complete {
                final_status = ExecutionStatus::AgentError;
                break;
            }

            if test_commands.is_empty() {
                info!("no test commands configured, skipping test validation");
                break;
            }

            let mut all_tests_passed = true;
            for cmd in &test_commands {
                let result = Self::run_test_command(repo_clone_path, cmd).await;
                if !result.passed {
                    all_tests_passed = false;
                }
                test_results.push(result);
            }

            if all_tests_passed {
                info!("all tests passed on iteration {}", iterations);
                final_status = ExecutionStatus::Success;
                break;
            }

            if iterations >= MAX_ITERATIONS {
                warn!(
                    "max iterations ({}) reached, tests still failing",
                    MAX_ITERATIONS
                );
                final_status = ExecutionStatus::MaxIterationsExceeded;
                break;
            }

            let fix_prompt = Self::build_fix_prompt(&test_results);
            jarvis
                .submit(Op::UserInput {
                    items: vec![UserInput::Text {
                        text: fix_prompt,
                        text_elements: vec![],
                    }],
                    final_output_json_schema: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("failed to submit fix prompt: {e}"))?;
        }

        let pr_url = if matches!(final_status, ExecutionStatus::Success) {
            Self::git_commit_and_push(repo_clone_path, &branch_name, &plan.commit_message).await?;
            let pr_request = jarvis_github::models::PullRequestCreateRequest {
                title: plan.pr_title.clone(),
                body: Some(plan.pr_body.clone()),
                head: branch_name.clone(),
                base: repo_context.default_branch.clone(),
                draft: None,
            };
            match github_client
                .create_pr(&repo_context.owner, &repo_context.repo, pr_request)
                .await
            {
                Ok(pr) => {
                    info!("created PR: {}", pr.html_url);
                    Some(pr.html_url)
                }
                Err(e) => {
                    warn!("push succeeded but create_pr failed: {e}");
                    None
                }
            }
        } else {
            None
        };

        Ok(ExecutionResult {
            status: final_status,
            branch_name,
            iterations,
            test_results,
            agent_summary: last_agent_summary,
            pr_url,
            error: None,
        })
    }

    fn build_implementation_prompt(
        plan: &ImplementationPlan,
        repo_context: &RepoContext,
    ) -> String {
        let steps_text = plan
            .steps
            .iter()
            .map(|s| {
                format!(
                    "{}. {} - {} file: {}",
                    s.step_number, s.description, s.change_type, s.file_path
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r##"## Issue to Implement
Summary: {}
Approach: {}

## Implementation Steps
{}

## Test Commands (run these to validate)
{}

## Repository Context
- Language: {}
- Framework: {}
- Patterns: {}

## Branch
Create and push branch: {}

## Commit Message
{}

## Instructions
1. Make the code changes as described in the steps
2. Run the test commands to validate
3. If tests fail, fix the issues and re-run tests
4. Once all tests pass, commit your changes with the commit message above
"##,
            plan.analysis.summary,
            plan.analysis.approach,
            steps_text,
            plan.test_commands.join("\n"),
            repo_context.language.as_deref().unwrap_or("unknown"),
            repo_context.framework.as_deref().unwrap_or("none"),
            repo_context.patterns.join(", "),
            plan.branch_name,
            plan.commit_message
        )
    }

    fn build_fix_prompt(test_results: &[TestRunResult]) -> String {
        let failures: Vec<String> = test_results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| format!("Command: {}\nOutput:\n{}", r.command, r.output))
            .collect();

        format!(
            r##"## Tests Failed

The following tests failed:

{}

## Instructions
1. Analyze the error output above
2. Fix the code to make the tests pass
3. Re-run the test commands
4. Report what you changed to fix the issue
"##,
            failures.join("\n\n---\n\n")
        )
    }

    async fn run_test_command(cwd: &PathBuf, command: &str) -> TestRunResult {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}{}", stdout, stderr);
                let truncated = if combined.len() > 10000 {
                    format!("{}...[truncated]", &combined[..10000])
                } else {
                    combined
                };

                TestRunResult {
                    command: command.to_string(),
                    passed: out.status.success(),
                    output: truncated,
                }
            }
            Err(e) => TestRunResult {
                command: command.to_string(),
                passed: false,
                output: format!("Failed to execute command: {e}"),
            },
        }
    }

    async fn git_commit_and_push(
        repo_path: &PathBuf,
        branch_name: &str,
        commit_message: &str,
    ) -> Result<()> {
        Self::run_git(repo_path, &["add", "-A"]).await?;
        Self::run_git(repo_path, &["commit", "-m", commit_message]).await?;
        Self::run_git(repo_path, &["push", "-u", "origin", branch_name]).await?;
        Ok(())
    }

    async fn run_git(repo_path: &PathBuf, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.args(args).current_dir(repo_path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git {:?} failed: {}", args, stderr);
        }

        Ok(())
    }
}

use jarvis_protocol::protocol::Op;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_implementation_prompt() {
        let plan = ImplementationPlan {
            analysis: crate::issue_resolver::types::IssueAnalysis {
                summary: "Add feature X".to_string(),
                complexity: crate::issue_resolver::types::IssueComplexity::Simple,
                category: crate::issue_resolver::types::IssueCategory::Feature,
                estimated_files: vec!["src/lib.rs".to_string()],
                approach: "Add function to lib.rs".to_string(),
                tests_needed: true,
                risks: vec![],
                can_auto_resolve: true,
                auto_resolve_reasoning: "Simple feature".to_string(),
                confidence: 0.9,
            },
            steps: vec![crate::issue_resolver::types::ImplementationStep {
                step_number: 1,
                description: "Add function".to_string(),
                file_path: "src/lib.rs".to_string(),
                change_type: "modify".to_string(),
                instructions: "Add the new function".to_string(),
                dependencies: vec![],
            }],
            branch_name: "jarvis/issue-123-add-feature".to_string(),
            commit_message: "Add feature X".to_string(),
            pr_title: "Add feature X".to_string(),
            pr_body: "Implements #123".to_string(),
            test_commands: vec!["cargo test".to_string()],
            confidence: 0.9,
        };

        let repo_context = RepoContext {
            owner: "test".to_string(),
            repo: "repo".to_string(),
            language: Some("Rust".to_string()),
            framework: None,
            relevant_files: vec![],
            tree_summary: "src/\ntests/".to_string(),
            readme: None,
            patterns: vec!["async".to_string()],
            default_branch: "main".to_string(),
        };

        let prompt = ImplementationExecutor::build_implementation_prompt(&plan, &repo_context);
        assert!(prompt.contains("Add feature X"));
        assert!(prompt.contains("jarvis/issue-123-add-feature"));
        assert!(prompt.contains("cargo test"));
    }

    #[test]
    fn test_build_fix_prompt() {
        let test_results = vec![TestRunResult {
            command: "cargo test".to_string(),
            passed: false,
            output: "error: expected something".to_string(),
        }];

        let prompt = ImplementationExecutor::build_fix_prompt(&test_results);
        assert!(prompt.contains("Tests Failed"));
        assert!(prompt.contains("expected something"));
    }
}
