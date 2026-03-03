//! Operations for GitHub issues.

use crate::client::GitHubClient;
use crate::errors::GitHubError;
use crate::models::Issue;
use crate::models::IssueCreateRequest;
use crate::models::IssueUpdateRequest;

/// Create a new issue in a repository.
pub async fn create_issue(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue: IssueCreateRequest,
) -> Result<Issue, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/issues");
    client.post(&path, &issue).await
}

/// Update an existing issue.
pub async fn update_issue(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue_number: u64,
    issue: IssueUpdateRequest,
) -> Result<Issue, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/issues/{issue_number}");
    client.patch(&path, &issue).await
}

/// List issues in a repository.
pub async fn list_issues(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    state: Option<&str>,
    labels: Option<&[&str]>,
) -> Result<Vec<Issue>, GitHubError> {
    let mut path = format!("/repos/{owner}/{repo}/issues");
    let mut query_params = Vec::new();

    if let Some(state) = state {
        query_params.push(format!("state={}", urlencoding::encode(state)));
    }

    if let Some(labels) = labels {
        let labels_str = labels
            .iter()
            .map(|l| urlencoding::encode(l))
            .collect::<Vec<_>>()
            .join(",");
        query_params.push(format!("labels={labels_str}"));
    }

    if !query_params.is_empty() {
        path.push('?');
        path.push_str(&query_params.join("&"));
    }

    client.get(&path).await
}
