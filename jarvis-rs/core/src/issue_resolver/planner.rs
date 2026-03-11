//! LLM-powered implementation planner.
//!
//! Uses the Jarvis sub-agent pattern to produce a detailed
//! [`ImplementationPlan`] from an [`IssueAnalysis`] and [`RepoContext`].

use std::sync::Arc;

use anyhow::Result;
use jarvis_github::models::Issue;
use jarvis_protocol::protocol::EventMsg;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

use crate::AuthManager;
use crate::Session;
use crate::TurnContext;
use crate::config::Config;
use crate::jarvis_delegate::run_codex_thread_one_shot;
use crate::models_manager::manager::ModelsManager;
use jarvis_protocol::user_input::UserInput;

use super::types::ImplementationPlan;
use super::types::IssueAnalysis;
use super::types::RepoContext;

/// Prompt template for implementation planning.
const PLANNER_PROMPT: &str = r###"You are an expert software engineer creating a detailed implementation plan.

## Repository Context
{repo_context}

## Issue
Title: {issue_title}
Body:
{issue_body}

## Analysis
{analysis_json}

## Relevant File Contents
{file_contents}

## Instructions

Based on the analysis above, create a detailed implementation plan.
Respond with ONLY a valid JSON object matching this schema:

```json
{
  "analysis": <the analysis object from above, copied as-is>,
  "steps": [
    {
      "step_number": 1,
      "description": "What to do in this step",
      "file_path": "path/to/file.rs",
      "change_type": "modify | create | delete",
      "instructions": "Detailed instructions for the change",
      "dependencies": []
    }
  ],
  "branch_name": "fix/issue-123-short-description",
  "commit_message": "fix: short description of the change",
  "pr_title": "Fix: Short description",
  "pr_body": "## Summary\n\nDescription of what this PR does.\n\nCloses #123",
  "test_commands": ["cargo test -p affected-crate"],
  "confidence": 0.85
}
```

Guidelines:
- Steps should be ordered by dependency (no step should depend on a later step).
- Use conventional commit format for commit messages.
- The PR body should reference the issue number.
- test_commands should be specific to the project's test framework.
- Be specific in instructions: include function names, struct names, etc.
- branch_name should follow the pattern: type/issue-NUMBER-short-description.
"###;

/// Produces a detailed [`ImplementationPlan`] using the LLM sub-agent pattern.
pub struct ImplementationPlanner;

impl ImplementationPlanner {
    /// Create an implementation plan for the given issue analysis.
    pub async fn plan(
        config: &Config,
        auth_manager: Arc<AuthManager>,
        models_manager: Arc<ModelsManager>,
        parent_session: Arc<Session>,
        parent_ctx: Arc<TurnContext>,
        cancel_token: CancellationToken,
        issue: &Issue,
        analysis: &IssueAnalysis,
        repo_context: &RepoContext,
    ) -> Result<ImplementationPlan> {
        let prompt = Self::build_prompt(issue, analysis, repo_context);

        let mut sub_config = config.clone();
        sub_config.base_instructions = Some(
            "You are an implementation planning engine. Respond with valid JSON only. No markdown fences, no explanation outside of JSON.".to_string(),
        );

        if let Some(model) = config.review_model.clone() {
            sub_config.model = Some(model);
        }

        let input = vec![UserInput::Text {
            text: prompt,
            text_elements: vec![],
        }];

        let jarvis = run_codex_thread_one_shot(
            sub_config,
            auth_manager,
            models_manager,
            input,
            parent_session,
            parent_ctx,
            cancel_token,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to start planner sub-agent: {e}"))?;

        let mut last_message: Option<String> = None;
        while let Ok(event) = jarvis.next_event().await {
            match event.msg {
                EventMsg::TurnComplete(tc) => {
                    last_message = tc.last_agent_message;
                    break;
                }
                EventMsg::TurnAborted(_) => {
                    return Err(anyhow::anyhow!("planning was aborted"));
                }
                _ => {}
            }
        }

        let text = last_message
            .ok_or_else(|| anyhow::anyhow!("planner sub-agent returned no response"))?;

        debug!("raw planner response: {text}");
        Self::parse_plan(&text)
    }

    fn build_prompt(issue: &Issue, analysis: &IssueAnalysis, repo_context: &RepoContext) -> String {
        let repo_ctx = format!(
            "Repository: {}/{}\nLanguage: {}\nFramework: {}\nDefault branch: {}\nPatterns: {}",
            repo_context.owner,
            repo_context.repo,
            repo_context.language.as_deref().unwrap_or("unknown"),
            repo_context.framework.as_deref().unwrap_or("none"),
            repo_context.default_branch,
            repo_context.patterns.join(", "),
        );

        let analysis_json =
            serde_json::to_string_pretty(analysis).unwrap_or_else(|_| "{}".to_string());

        let file_contents = repo_context
            .relevant_files
            .iter()
            .filter_map(|f| {
                f.content
                    .as_ref()
                    .map(|c| format!("### {}\n```\n{}\n```", f.path, c))
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let issue_body = issue.body.as_deref().unwrap_or("(no body)");

        PLANNER_PROMPT
            .replace("{repo_context}", &repo_ctx)
            .replace("{issue_title}", &issue.title)
            .replace("{issue_body}", issue_body)
            .replace("{analysis_json}", &analysis_json)
            .replace("{file_contents}", &file_contents)
    }

    fn parse_plan(text: &str) -> Result<ImplementationPlan> {
        if let Ok(plan) = serde_json::from_str::<ImplementationPlan>(text) {
            return Ok(plan);
        }

        if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
            if start < end {
                if let Some(slice) = text.get(start..=end) {
                    if let Ok(plan) = serde_json::from_str::<ImplementationPlan>(slice) {
                        return Ok(plan);
                    }
                }
            }
        }

        warn!("failed to parse plan JSON, raw text: {text}");
        Err(anyhow::anyhow!(
            "could not parse LLM response as ImplementationPlan"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue_resolver::types::IssueCategory;
    use crate::issue_resolver::types::IssueComplexity;

    fn sample_analysis() -> IssueAnalysis {
        IssueAnalysis {
            summary: "Fix typo".to_string(),
            complexity: IssueComplexity::Trivial,
            category: IssueCategory::Documentation,
            estimated_files: vec!["README.md".to_string()],
            approach: "Fix the typo".to_string(),
            tests_needed: false,
            risks: vec![],
            can_auto_resolve: true,
            auto_resolve_reasoning: "Simple typo".to_string(),
            confidence: 0.95,
        }
    }

    #[test]
    fn test_parse_plan_valid_json() {
        let analysis = sample_analysis();
        let analysis_json = serde_json::to_string(&analysis).unwrap();

        let json = format!(
            r#"{{
            "analysis": {analysis_json},
            "steps": [
                {{
                    "step_number": 1,
                    "description": "Fix typo in README",
                    "file_path": "README.md",
                    "change_type": "modify",
                    "instructions": "Change 'teh' to 'the' on line 5",
                    "dependencies": []
                }}
            ],
            "branch_name": "fix/issue-1-typo",
            "commit_message": "fix: correct typo in README",
            "pr_title": "Fix: Correct typo in README",
            "pr_body": "Closes #1",
            "test_commands": [],
            "confidence": 0.95
        }}"#
        );

        let plan = ImplementationPlanner::parse_plan(&json).unwrap();
        assert_eq!(plan.branch_name, "fix/issue-1-typo");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].file_path, "README.md");
    }

    #[test]
    fn test_parse_plan_invalid() {
        assert!(ImplementationPlanner::parse_plan("not json").is_err());
    }
}
