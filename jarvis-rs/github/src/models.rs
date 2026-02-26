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
