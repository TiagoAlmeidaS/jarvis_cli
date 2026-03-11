//! LLM-powered issue analyzer.
//!
//! Uses the Jarvis sub-agent pattern (`run_codex_thread_one_shot`) to analyze
//! a GitHub issue and produce a structured [`IssueAnalysis`].

use std::sync::Arc;

use anyhow::Result;
use jarvis_github::models::Issue;
use jarvis_github::models::IssueComment;
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

use super::types::IssueAnalysis;
use super::types::RepoContext;

/// Prompt template for issue analysis.
///
/// The LLM is instructed to return valid JSON matching [`IssueAnalysis`].
const ANALYSIS_PROMPT: &str = r#"You are an expert software engineer specializing in automated issue resolution.

Analyze the following GitHub issue and produce a structured assessment.

## Repository Context
{repo_context}

## Issue
Title: {issue_title}
Body:
{issue_body}

## Comments
{issue_comments}

## Instructions

Analyze this issue and respond with ONLY a valid JSON object matching this schema:

```json
{
  "summary": "Short summary of what the issue is asking for",
  "complexity": "trivial | simple | moderate | complex",
  "category": "bug_fix | feature | refactor | documentation | test | chore | security",
  "estimated_files": ["list", "of", "files", "likely", "to", "change"],
  "approach": "High-level approach to resolve the issue",
  "tests_needed": true,
  "risks": ["list of identified risks"],
  "can_auto_resolve": true,
  "auto_resolve_reasoning": "Why this can/cannot be auto-resolved",
  "confidence": 0.85
}
```

Guidelines:
- Set `can_auto_resolve` to `true` only for trivial or simple issues that are well-defined.
- Issues involving security, database schema changes, or major refactoring should NOT be auto-resolved.
- Confidence should reflect how well you understand what needs to be done (0.0 to 1.0).
- estimated_files should be realistic paths based on the repository structure provided.
- Be conservative: when in doubt, set `can_auto_resolve` to `false`.
"#;

/// Analyzes a GitHub issue using the LLM sub-agent pattern.
pub struct IssueAnalyzer;

impl IssueAnalyzer {
    /// Analyze an issue and produce a structured [`IssueAnalysis`].
    ///
    /// This follows the `ReviewTask` pattern: spawn a sub-agent via
    /// `run_codex_thread_one_shot`, listen for events until `TurnComplete`,
    /// and parse the last agent message as JSON.
    pub async fn analyze(
        config: &Config,
        auth_manager: Arc<AuthManager>,
        models_manager: Arc<ModelsManager>,
        parent_session: Arc<Session>,
        parent_ctx: Arc<TurnContext>,
        cancel_token: CancellationToken,
        issue: &Issue,
        comments: &[IssueComment],
        repo_context: &RepoContext,
    ) -> Result<IssueAnalysis> {
        let prompt = Self::build_prompt(issue, comments, repo_context);

        // Configure sub-agent with analysis-specific instructions.
        let mut sub_config = config.clone();
        sub_config.base_instructions = Some(
            "You are an issue analysis engine. Respond with valid JSON only. No markdown fences, no explanation outside of JSON.".to_string(),
        );

        // Use the review model if configured, otherwise default model.
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
            cancel_token.clone(),
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to start analysis sub-agent: {e}"))?;

        // Process events until we get TurnComplete with the agent's response.
        let mut last_message: Option<String> = None;
        while let Ok(event) = jarvis.next_event().await {
            match event.msg {
                EventMsg::TurnComplete(tc) => {
                    last_message = tc.last_agent_message;
                    break;
                }
                EventMsg::TurnAborted(_) => {
                    return Err(anyhow::anyhow!("analysis was aborted"));
                }
                _ => {
                    // Ignore other events (deltas, approvals forwarded by delegate, etc.)
                }
            }
        }

        let text = last_message
            .ok_or_else(|| anyhow::anyhow!("analysis sub-agent returned no response"))?;

        debug!("raw analysis response: {text}");
        Self::parse_analysis(&text)
    }

    fn build_prompt(
        issue: &Issue,
        comments: &[IssueComment],
        repo_context: &RepoContext,
    ) -> String {
        let repo_ctx = format!(
            "Repository: {}/{}\nLanguage: {}\nFramework: {}\nPatterns: {}\nTree:\n{}",
            repo_context.owner,
            repo_context.repo,
            repo_context.language.as_deref().unwrap_or("unknown"),
            repo_context.framework.as_deref().unwrap_or("none"),
            repo_context.patterns.join(", "),
            truncate_string(&repo_context.tree_summary, 2000),
        );

        let issue_body = issue.body.as_deref().unwrap_or("(no body)");

        let comments_text = if comments.is_empty() {
            "(no comments)".to_string()
        } else {
            comments
                .iter()
                .map(|c| format!("@{}: {}", c.user.login, c.body))
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        ANALYSIS_PROMPT
            .replace("{repo_context}", &repo_ctx)
            .replace("{issue_title}", &issue.title)
            .replace("{issue_body}", issue_body)
            .replace("{issue_comments}", &comments_text)
    }

    fn parse_analysis(text: &str) -> Result<IssueAnalysis> {
        // Try direct parse first.
        if let Ok(analysis) = serde_json::from_str::<IssueAnalysis>(text) {
            return Ok(analysis);
        }

        // Try extracting JSON from markdown fences or surrounding text.
        if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
            if start < end {
                if let Some(slice) = text.get(start..=end) {
                    if let Ok(analysis) = serde_json::from_str::<IssueAnalysis>(slice) {
                        return Ok(analysis);
                    }
                }
            }
        }

        warn!("failed to parse analysis JSON, raw text: {text}");
        Err(anyhow::anyhow!(
            "could not parse LLM response as IssueAnalysis"
        ))
    }
}

/// Truncate a string to at most `max_len` characters, appending "..." if truncated.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue_resolver::types::IssueCategory;
    use crate::issue_resolver::types::IssueComplexity;

    #[test]
    fn test_parse_analysis_direct_json() {
        let json = r#"{
            "summary": "Fix a typo in README",
            "complexity": "trivial",
            "category": "documentation",
            "estimated_files": ["README.md"],
            "approach": "Fix the typo",
            "tests_needed": false,
            "risks": [],
            "can_auto_resolve": true,
            "auto_resolve_reasoning": "Simple typo fix",
            "confidence": 0.95
        }"#;

        let analysis = IssueAnalyzer::parse_analysis(json).unwrap();
        assert_eq!(analysis.summary, "Fix a typo in README");
        assert_eq!(analysis.complexity, IssueComplexity::Trivial);
        assert_eq!(analysis.category, IssueCategory::Documentation);
        assert!(analysis.can_auto_resolve);
        assert_eq!(analysis.confidence, 0.95);
    }

    #[test]
    fn test_parse_analysis_with_markdown_fences() {
        let text = r#"Here is my analysis:

```json
{
    "summary": "Add logging",
    "complexity": "simple",
    "category": "feature",
    "estimated_files": ["src/lib.rs"],
    "approach": "Add tracing calls",
    "tests_needed": true,
    "risks": ["may affect performance"],
    "can_auto_resolve": true,
    "auto_resolve_reasoning": "Well-defined feature",
    "confidence": 0.8
}
```"#;

        let analysis = IssueAnalyzer::parse_analysis(text).unwrap();
        assert_eq!(analysis.summary, "Add logging");
        assert_eq!(analysis.complexity, IssueComplexity::Simple);
    }

    #[test]
    fn test_parse_analysis_invalid() {
        let text = "This is not JSON at all";
        assert!(IssueAnalyzer::parse_analysis(text).is_err());
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 5), "hello...");
    }
}
