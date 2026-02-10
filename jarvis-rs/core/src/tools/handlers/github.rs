//! GitHub tool handlers for creating issues, commenting on PRs, and managing repositories.

use async_trait::async_trait;
use jarvis_github::GitHubClient;
use jarvis_github::GitHubError;
use jarvis_protocol::ThreadId;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

use crate::config::Config;
use crate::function_tool::FunctionCallError;
use crate::TurnContext;
use jarvis_secrets::SecretName;
use jarvis_secrets::SecretScope;
use jarvis_secrets::SecretsManager;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

/// Handler for GitHub operations.
pub struct GitHubHandler;

#[derive(Debug, Deserialize)]
struct CreateIssueParams {
    owner: String,
    repo: String,
    title: String,
    body: Option<String>,
    labels: Option<Vec<String>>,
    assignees: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CommentPRParams {
    owner: String,
    repo: String,
    pr_number: u64,
    comment: String,
}

#[derive(Debug, Deserialize)]
struct ListReposParams {
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloneRepoParams {
    owner: String,
    repo: String,
    use_ssh: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ListIssuesParams {
    owner: String,
    repo: String,
    state: Option<String>,
    labels: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct GitHubToolOutput {
    success: bool,
    message: String,
    data: Option<serde_json::Value>,
}

impl GitHubHandler {
    /// Get or create a GitHub client using PAT from environment variable or secrets.
    /// 
    /// Priority order:
    /// 1. Environment variable `GITHUB_PAT` or `jarvis_GITHUB_PAT`
    /// 2. Secrets manager using configured secret name
    async fn get_client(
        config: &Config,
        secrets_manager: &SecretsManager,
    ) -> Result<GitHubClient, FunctionCallError> {
        // Try environment variables first (GITHUB_PAT takes precedence over jarvis_GITHUB_PAT)
        let pat = std::env::var("GITHUB_PAT")
            .or_else(|_| std::env::var("jarvis_GITHUB_PAT"))
            .ok()
            .filter(|pat| !pat.trim().is_empty())
            .map(Ok)
            .unwrap_or_else(|| {
                // Fall back to secrets manager if no valid env var found
                Self::get_pat_from_secrets(config, secrets_manager)
            })?;

        // Try environment variable for API base URL, fall back to config
        let api_base_url = std::env::var("GITHUB_API_BASE_URL")
            .or_else(|_| std::env::var("jarvis_GITHUB_API_BASE_URL"))
            .unwrap_or_else(|_| config.github.api_base_url.clone());

        GitHubClient::with_base_url(pat, api_base_url).map_err(|e| {
            FunctionCallError::RespondToModel(format!("Failed to create GitHub client: {e}"))
        })
    }

    /// Get PAT from secrets manager.
    fn get_pat_from_secrets(
        config: &Config,
        secrets_manager: &SecretsManager,
    ) -> Result<String, FunctionCallError> {
        let secret_name = SecretName::new(&config.github.pat_secret_name)
            .map_err(|e| FunctionCallError::RespondToModel(format!("Invalid secret name: {e}")))?;

        let scope = SecretScope::Global;
        secrets_manager
            .get(&scope, &secret_name)
            .map_err(|e| {
                FunctionCallError::RespondToModel(format!("Failed to retrieve GitHub PAT: {e}"))
            })?
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(format!(
                    "GitHub PAT not found. Please set it using:\n  - Environment variable: GITHUB_PAT or jarvis_GITHUB_PAT\n  - Or secrets: jarvis secrets set {} <token>",
                    config.github.pat_secret_name
                ))
            })
    }

    async fn handle_create_issue(
        params: CreateIssueParams,
        config: &Config,
        secrets_manager: &SecretsManager,
    ) -> Result<ToolOutput, FunctionCallError> {
        let client = Self::get_client(config, secrets_manager).await?;

        let issue_request = jarvis_github::models::IssueCreateRequest {
            title: params.title,
            body: params.body,
            labels: params.labels,
            assignees: params.assignees,
        };

        let issue = client
            .create_issue(&params.owner, &params.repo, issue_request)
            .await
            .map_err(|e| Self::format_error("create_issue", e))?;

        let output = GitHubToolOutput {
            success: true,
            message: format!("Created issue #{}: {}", issue.number, issue.title),
            data: Some(serde_json::to_value(&issue).unwrap()),
        };
        Ok(ToolOutput::Function {
            content: serde_json::to_string(&output).unwrap(),
            content_items: None,
            success: Some(true),
        })
    }

    async fn handle_comment_pr(
        params: CommentPRParams,
        config: &Config,
        secrets_manager: &SecretsManager,
    ) -> Result<ToolOutput, FunctionCallError> {
        let client = Self::get_client(config, secrets_manager).await?;

        let comment = client
            .comment_pr(&params.owner, &params.repo, params.pr_number, params.comment)
            .await
            .map_err(|e| Self::format_error("comment_pr", e))?;

        let output = GitHubToolOutput {
            success: true,
            message: format!("Commented on PR #{}", params.pr_number),
            data: Some(serde_json::to_value(&comment).unwrap()),
        };
        Ok(ToolOutput::Function {
            content: serde_json::to_string(&output).unwrap(),
            content_items: None,
            success: Some(true),
        })
    }

    async fn handle_list_repos(
        params: ListReposParams,
        config: &Config,
        secrets_manager: &SecretsManager,
    ) -> Result<ToolOutput, FunctionCallError> {
        let client = Self::get_client(config, secrets_manager).await?;

        let repos = client
            .list_repositories(params.username.as_deref())
            .await
            .map_err(|e| Self::format_error("list_repos", e))?;

        let output = GitHubToolOutput {
            success: true,
            message: format!("Found {} repositories", repos.len()),
            data: Some(serde_json::to_value(&repos).unwrap()),
        };
        Ok(ToolOutput::Function {
            content: serde_json::to_string(&output).unwrap(),
            content_items: None,
            success: Some(true),
        })
    }

    async fn handle_clone_repo(
        params: CloneRepoParams,
        config: &Config,
        secrets_manager: &SecretsManager,
    ) -> Result<ToolOutput, FunctionCallError> {
        let client = Self::get_client(config, secrets_manager).await?;

        let clone_url = client
            .clone_repo(&params.owner, &params.repo, params.use_ssh.unwrap_or(false))
            .await
            .map_err(|e| Self::format_error("clone_repo", e))?;

        let output = GitHubToolOutput {
            success: true,
            message: format!("Clone URL for {}/{}: {}", params.owner, params.repo, clone_url),
            data: Some(serde_json::json!({ "clone_url": clone_url })),
        };
        Ok(ToolOutput::Function {
            content: serde_json::to_string(&output).unwrap(),
            content_items: None,
            success: Some(true),
        })
    }

    async fn handle_list_issues(
        params: ListIssuesParams,
        config: &Config,
        secrets_manager: &SecretsManager,
    ) -> Result<ToolOutput, FunctionCallError> {
        let client = Self::get_client(config, secrets_manager).await?;

        let labels_vec: Option<Vec<&str>> = params
            .labels
            .as_ref()
            .map(|labels| labels.iter().map(|s| s.as_str()).collect());
        let labels_ref: Option<&[&str]> = labels_vec.as_ref().map(|v| v.as_slice());

        let issues = client
            .list_issues(
                &params.owner,
                &params.repo,
                params.state.as_deref(),
                labels_ref,
            )
            .await
            .map_err(|e| Self::format_error("list_issues", e))?;

        let output = GitHubToolOutput {
            success: true,
            message: format!("Found {} issues", issues.len()),
            data: Some(serde_json::to_value(&issues).unwrap()),
        };
        Ok(ToolOutput::Function {
            content: serde_json::to_string(&output).unwrap(),
            content_items: None,
            success: Some(true),
        })
    }

    fn format_error(operation: &str, error: GitHubError) -> FunctionCallError {
        let message = match error {
            GitHubError::Authentication(msg) => {
                format!("GitHub authentication failed for {operation}: {msg}. Please check your PAT token.")
            }
            GitHubError::RateLimit { reset_at } => {
                format!(
                    "GitHub API rate limit exceeded for {operation}. Reset at: {:?}",
                    reset_at
                )
            }
            GitHubError::Api { status, message } => {
                format!("GitHub API error for {operation} (status {}): {message}", status)
            }
            _ => format!("GitHub error for {operation}: {error}"),
        };
        FunctionCallError::RespondToModel(message)
    }
}

#[async_trait]
impl ToolHandler for GitHubHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        matches!(payload, ToolPayload::Function { .. })
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            tool_name,
            payload,
            turn,
            ..
        } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "GitHub tools only support function calls".to_string(),
                ))
            }
        };

        let config = turn.config.as_ref();
        // Get secrets manager from config
        let secrets_manager = SecretsManager::new(
            config.jarvis_home.clone(),
            jarvis_secrets::SecretsBackendKind::Local,
        );

        match tool_name.as_str() {
            "github_create_issue" => {
                let params: CreateIssueParams = parse_arguments(&arguments)?;
                Self::handle_create_issue(params, config, &secrets_manager).await
            }
            "github_comment_pr" => {
                let params: CommentPRParams = parse_arguments(&arguments)?;
                Self::handle_comment_pr(params, config, &secrets_manager).await
            }
            "github_list_repos" => {
                let params: ListReposParams = parse_arguments(&arguments)?;
                Self::handle_list_repos(params, config, &secrets_manager).await
            }
            "github_clone_repo" => {
                let params: CloneRepoParams = parse_arguments(&arguments)?;
                Self::handle_clone_repo(params, config, &secrets_manager).await
            }
            "github_list_issues" => {
                let params: ListIssuesParams = parse_arguments(&arguments)?;
                Self::handle_list_issues(params, config, &secrets_manager).await
            }
            _ => Err(FunctionCallError::RespondToModel(format!(
                "Unknown GitHub tool: {tool_name}"
            ))),
        }
    }
}
