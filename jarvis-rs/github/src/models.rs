//! Data models for GitHub API operations.

use serde::Deserialize;
use serde::Serialize;

/// A GitHub issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub user: User,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

/// A label on a GitHub issue or PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

/// A GitHub user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub login: String,
    pub avatar_url: Option<String>,
}

/// Request to create a new issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCreateRequest {
    pub title: String,
    pub body: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

/// Request to update an issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueUpdateRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

/// A GitHub pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: User,
    pub head: Branch,
    pub base: Branch,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

/// A branch reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub ref_field: String,
    pub sha: String,
    #[serde(rename = "repo")]
    pub repository: Option<Repository>,
}

/// A comment on a pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRComment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

/// Request to create a PR comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRCommentCreate {
    pub body: String,
}

/// A GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub fork: bool,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: Option<String>,
    pub default_branch: String,
    pub owner: User,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for listing repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryListResponse {
    pub repositories: Vec<Repository>,
    pub total_count: Option<u64>,
}

/// A comment on a GitHub issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

/// Request to create a pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestCreateRequest {
    pub title: String,
    pub body: Option<String>,
    pub head: String,
    pub base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<bool>,
}

/// A Git reference (branch, tag, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub node_id: String,
    pub url: String,
    pub object: GitObject,
}

/// A Git object (commit, tree, blob, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitObject {
    #[serde(rename = "type")]
    pub object_type: String,
    pub sha: String,
    pub url: String,
}

/// Request to create a Git reference.
#[derive(Debug, Clone, Serialize)]
pub struct GitRefCreateRequest {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// A tree entry in a Git repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub sha: String,
    pub size: Option<u64>,
    pub url: Option<String>,
}

/// A Git tree response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTree {
    pub sha: String,
    pub url: String,
    pub tree: Vec<TreeEntry>,
    pub truncated: bool,
}

/// File content response from GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: u64,
    #[serde(rename = "type")]
    pub content_type: String,
    pub content: Option<String>,
    pub encoding: Option<String>,
    pub html_url: String,
    pub download_url: Option<String>,
}

impl FileContent {
    /// Decode base64-encoded content returned by the GitHub API.
    pub fn decoded_content(&self) -> Option<String> {
        self.content.as_ref().and_then(|c| {
            let cleaned: String = c.chars().filter(|ch| !ch.is_whitespace()).collect();
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(cleaned)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }
}
