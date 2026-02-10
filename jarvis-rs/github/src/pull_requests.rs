//! Operations for GitHub pull requests.

use crate::client::GitHubClient;
use crate::errors::GitHubError;
use crate::models::{PRComment, PRCommentCreate, PullRequest};

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
