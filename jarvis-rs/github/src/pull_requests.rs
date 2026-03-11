//! Operations for GitHub pull requests.

use crate::client::GitHubClient;
use crate::errors::GitHubError;
use crate::models::PRComment;
use crate::models::PRCommentCreate;
use crate::models::PullRequest;
use crate::models::PullRequestCreateRequest;

/// Get a pull request by number.
pub async fn get_pr(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<PullRequest, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/pulls/{pr_number}");
    client.get(&path).await
}

/// Create a new pull request.
pub async fn create_pr(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr: PullRequestCreateRequest,
) -> Result<PullRequest, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/pulls");
    client.post(&path, &pr).await
}

/// List pull requests in a repository.
pub async fn list_prs(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    state: Option<&str>,
) -> Result<Vec<PullRequest>, GitHubError> {
    let mut path = format!("/repos/{owner}/{repo}/pulls");
    if let Some(state) = state {
        path.push_str(&format!("?state={}", urlencoding::encode(state)));
    }
    client.get(&path).await
}

/// Create a comment on a pull request.
pub async fn comment_pr(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
    comment: String,
) -> Result<PRComment, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/issues/{pr_number}/comments");
    let comment_request = PRCommentCreate { body: comment };
    client.post(&path, &comment_request).await
}

/// List comments on a pull request.
pub async fn list_pr_comments(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<Vec<PRComment>, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/issues/{pr_number}/comments");
    client.get(&path).await
}
