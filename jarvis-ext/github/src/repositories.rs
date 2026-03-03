//! Operations for GitHub repositories.

use crate::client::GitHubClient;
use crate::errors::GitHubError;
use crate::models::Repository;

/// List repositories for a user or organization.
/// If username is None, lists repositories for the authenticated user.
pub async fn list_repositories(
    client: &GitHubClient,
    username: Option<&str>,
) -> Result<Vec<Repository>, GitHubError> {
    let path = if let Some(username) = username {
        format!("/users/{username}/repos")
    } else {
        "/user/repos".to_string()
    };
    client.get(&path).await
}

/// Get repository information.
pub async fn get_repo(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<Repository, GitHubError> {
    let path = format!("/repos/{owner}/{repo}");
    client.get(&path).await
}

/// Get clone URL for a repository.
/// Returns the SSH URL if use_ssh is true, otherwise the HTTPS URL.
pub async fn clone_repo(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    use_ssh: bool,
) -> Result<String, GitHubError> {
    let repository = get_repo(client, owner, repo).await?;
    Ok(if use_ssh {
        repository
            .ssh_url
            .unwrap_or_else(|| repository.clone_url.clone())
    } else {
        repository.clone_url
    })
}
