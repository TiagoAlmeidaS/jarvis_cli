//! GitHub API client implementation.

use crate::errors::GitHubError;
use crate::git::create_ref;
use crate::git::get_branch_sha;
use crate::git::get_file_content;
use crate::git::get_tree;
use crate::issues::close_issue;
use crate::issues::create_issue;
use crate::issues::get_issue;
use crate::issues::list_issue_comments;
use crate::issues::list_issues;
use crate::issues::update_issue;
use crate::models::FileContent;
use crate::models::GitRef;
use crate::models::GitTree;
use crate::models::Issue;
use crate::models::IssueComment;
use crate::models::IssueCreateRequest;
use crate::models::IssueUpdateRequest;
use crate::models::PRComment;
use crate::models::PullRequest;
use crate::models::PullRequestCreateRequest;
use crate::models::Repository;
use crate::pull_requests::comment_pr;
use crate::pull_requests::create_pr;
use crate::pull_requests::get_pr;
use crate::pull_requests::list_pr_comments;
use crate::pull_requests::list_prs;
use crate::repositories::clone_repo;
use crate::repositories::get_repo;
use crate::repositories::list_repositories;
use http::header::AUTHORIZATION;
use http::header::HeaderValue;
use reqwest::Client;
use std::time::Duration;

const DEFAULT_API_BASE_URL: &str = "https://api.github.com";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Client for interacting with the GitHub API.
#[derive(Debug)]
pub struct GitHubClient {
    client: Client,
    base_url: String,
    pat: String,
}

impl GitHubClient {
    /// Create a new GitHub client with a Personal Access Token.
    pub fn new(pat: String) -> Result<Self, GitHubError> {
        Self::with_base_url(pat, DEFAULT_API_BASE_URL.to_string())
    }

    /// Create a new GitHub client with a custom base URL (useful for GitHub Enterprise).
    pub fn with_base_url(pat: String, base_url: String) -> Result<Self, GitHubError> {
        if pat.trim().is_empty() {
            return Err(GitHubError::Authentication(
                "PAT token cannot be empty".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("jarvis-github/1.0")
            .build()
            .map_err(|e| GitHubError::Other(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self {
            client,
            base_url,
            pat,
        })
    }

    /// Get the base URL for API requests.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Create an authorization header value.
    fn auth_header(&self) -> HeaderValue {
        HeaderValue::from_str(&format!("token {}", self.pat))
            .expect("PAT should be valid header value")
    }

    /// Make a GET request to the GitHub API.
    pub(crate) async fn get<T>(&self, path: &str) -> Result<T, GitHubError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        tracing::debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Make a POST request to the GitHub API.
    pub(crate) async fn post<T>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, GitHubError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        tracing::debug!("POST {}", url);

        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .json(body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Make a PATCH request to the GitHub API.
    pub(crate) async fn patch<T>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, GitHubError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        tracing::debug!("PATCH {}", url);

        let response = self
            .client
            .patch(&url)
            .header(AUTHORIZATION, self.auth_header())
            .header("Accept", "application/vnd.github.v3+json")
            .json(body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Handle an HTTP response, checking for errors and rate limits.
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T, GitHubError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        // Check rate limit headers
        if let Some(reset_at) = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            if status.as_u16() == 403 {
                return Err(GitHubError::RateLimit {
                    reset_at: Some(reset_at),
                });
            }
        }

        // Handle authentication errors
        if status.as_u16() == 401 {
            return Err(GitHubError::Authentication(
                "Invalid or expired PAT token".to_string(),
            ));
        }

        // Handle other errors
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GitHubError::from_response(status.as_u16(), body));
        }

        // Parse successful response
        let body = response.text().await?;
        serde_json::from_str(&body).map_err(GitHubError::Serialization)
    }

    /// Create a new issue in a repository.
    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        issue: IssueCreateRequest,
    ) -> Result<Issue, GitHubError> {
        create_issue(self, owner, repo, issue).await
    }

    /// Update an existing issue.
    pub async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        issue: IssueUpdateRequest,
    ) -> Result<Issue, GitHubError> {
        update_issue(self, owner, repo, issue_number, issue).await
    }

    /// List issues in a repository.
    pub async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
        labels: Option<&[&str]>,
    ) -> Result<Vec<Issue>, GitHubError> {
        list_issues(self, owner, repo, state, labels).await
    }

    /// Get a pull request by number.
    pub async fn get_pr(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest, GitHubError> {
        get_pr(self, owner, repo, pr_number).await
    }

    /// Create a comment on a pull request.
    pub async fn comment_pr(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        comment: String,
    ) -> Result<PRComment, GitHubError> {
        comment_pr(self, owner, repo, pr_number, comment).await
    }

    /// List comments on a pull request.
    pub async fn list_pr_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<PRComment>, GitHubError> {
        list_pr_comments(self, owner, repo, pr_number).await
    }

    /// List repositories for a user or organization.
    pub async fn list_repositories(
        &self,
        username: Option<&str>,
    ) -> Result<Vec<Repository>, GitHubError> {
        list_repositories(self, username).await
    }

    /// Get repository information.
    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<Repository, GitHubError> {
        get_repo(self, owner, repo).await
    }

    /// Clone a repository (returns clone URL).
    pub async fn clone_repo(
        &self,
        owner: &str,
        repo: &str,
        use_ssh: bool,
    ) -> Result<String, GitHubError> {
        clone_repo(self, owner, repo, use_ssh).await
    }

    /// Get a single issue by number.
    pub async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Issue, GitHubError> {
        get_issue(self, owner, repo, issue_number).await
    }

    /// Close an issue.
    pub async fn close_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Issue, GitHubError> {
        close_issue(self, owner, repo, issue_number).await
    }

    /// List comments on an issue.
    pub async fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Vec<IssueComment>, GitHubError> {
        list_issue_comments(self, owner, repo, issue_number).await
    }

    /// Create a new pull request.
    pub async fn create_pr(
        &self,
        owner: &str,
        repo: &str,
        pr: PullRequestCreateRequest,
    ) -> Result<PullRequest, GitHubError> {
        create_pr(self, owner, repo, pr).await
    }

    /// List pull requests in a repository.
    pub async fn list_prs(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
    ) -> Result<Vec<PullRequest>, GitHubError> {
        list_prs(self, owner, repo, state).await
    }

    /// Create a new Git reference (branch).
    pub async fn create_branch(
        &self,
        owner: &str,
        repo: &str,
        branch_name: &str,
        from_sha: &str,
    ) -> Result<GitRef, GitHubError> {
        create_ref(self, owner, repo, branch_name, from_sha).await
    }

    /// Get the SHA of a branch.
    pub async fn get_branch_sha(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<String, GitHubError> {
        get_branch_sha(self, owner, repo, branch).await
    }

    /// Get the tree of a repository (recursive).
    pub async fn get_tree(
        &self,
        owner: &str,
        repo: &str,
        tree_sha: &str,
    ) -> Result<GitTree, GitHubError> {
        get_tree(self, owner, repo, tree_sha).await
    }

    /// Get file content from a repository.
    pub async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        file_path: &str,
        git_ref: Option<&str>,
    ) -> Result<FileContent, GitHubError> {
        get_file_content(self, owner, repo, file_path, git_ref).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pat_rejected() {
        let result = GitHubClient::new(String::new());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GitHubError::Authentication(_)
        ));
    }
}
