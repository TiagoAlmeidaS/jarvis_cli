//! Operations for GitHub Git API (refs, trees, blobs).

use crate::client::GitHubClient;
use crate::errors::GitHubError;
use crate::models::FileContent;
use crate::models::GitRef;
use crate::models::GitRefCreateRequest;
use crate::models::GitTree;

/// Create a new Git reference (branch).
pub async fn create_ref(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    ref_name: &str,
    sha: &str,
) -> Result<GitRef, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/git/refs");
    let request = GitRefCreateRequest {
        ref_name: format!("refs/heads/{ref_name}"),
        sha: sha.to_string(),
    };
    client.post(&path, &request).await
}

/// Get the SHA of a branch (for creating refs from).
pub async fn get_branch_sha(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<String, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/git/ref/heads/{branch}");
    let git_ref: GitRef = client.get(&path).await?;
    Ok(git_ref.object.sha)
}

/// Get the tree of a repository (recursive).
pub async fn get_tree(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    tree_sha: &str,
) -> Result<GitTree, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/git/trees/{tree_sha}?recursive=1");
    client.get(&path).await
}

/// Get file content from a repository.
pub async fn get_file_content(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    file_path: &str,
    git_ref: Option<&str>,
) -> Result<FileContent, GitHubError> {
    let encoded_path = urlencoding::encode(file_path);
    let mut path = format!("/repos/{owner}/{repo}/contents/{encoded_path}");
    if let Some(git_ref) = git_ref {
        path.push_str(&format!("?ref={}", urlencoding::encode(git_ref)));
    }
    client.get(&path).await
}
