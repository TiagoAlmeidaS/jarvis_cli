# Plan: Jarvis Autonomous Issue Resolver

## Overview

Build an end-to-end autonomous pipeline that monitors GitHub repositories, picks up issues,
implements solutions via LLM, runs tests, and creates PRs -- all without human intervention
(with optional safety gates for complex changes).

Target: **any repo** in the user's GitHub account.
Initial scope: **small features** (new endpoints, components, etc.).

---

## FASE 1 -- Complete GitHub API Client

### 1A. New models in `jarvis-rs/github/src/models.rs`

Add after `RepositoryListResponse`:

```rust
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
                .decode(&cleaned)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }
}
```

Note: Add `base64` dependency to `jarvis-rs/github/Cargo.toml`.

### 1B. New methods in `jarvis-rs/github/src/issues.rs`

Add after `list_issues`:

```rust
/// Get a single issue by number.
pub async fn get_issue(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<Issue, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/issues/{issue_number}");
    client.get(&path).await
}

/// Close an issue.
pub async fn close_issue(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<Issue, GitHubError> {
    let update = IssueUpdateRequest {
        title: None,
        body: None,
        state: Some("closed".to_string()),
        labels: None,
        assignees: None,
    };
    update_issue(client, owner, repo, issue_number, update).await
}

/// List comments on an issue.
pub async fn list_issue_comments(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<Vec<IssueComment>, GitHubError> {
    let path = format!("/repos/{owner}/{repo}/issues/{issue_number}/comments");
    client.get(&path).await
}
```

### 1C. New methods in `jarvis-rs/github/src/pull_requests.rs`

Add after `list_pr_comments`:

```rust
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
```

### 1D. New file: `jarvis-rs/github/src/git.rs`

```rust
//! Operations for GitHub Git API (refs, trees, blobs).

use crate::client::GitHubClient;
use crate::errors::GitHubError;
use crate::models::{FileContent, GitRef, GitRefCreateRequest, GitTree};

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
```

### 1E. Update `jarvis-rs/github/src/lib.rs`

```rust
pub mod git;  // <-- add this line
```

### 1F. Update `jarvis-rs/github/src/client.rs`

Add imports and delegation methods for:

- `get_issue`, `close_issue`, `list_issue_comments` (from issues module)
- `create_pr`, `list_prs` (from pull_requests module)
- `create_branch`, `get_branch_sha`, `get_tree`, `get_file_content` (from git module)

### 1G. Register new LLM Tools

In `jarvis-rs/core/src/tools/spec.rs`, add tool spec functions:

- `create_github_get_issue_tool()`
- `create_github_close_issue_tool()`
- `create_github_list_issue_comments_tool()`
- `create_github_create_pr_tool()`
- `create_github_create_branch_tool()`
- `create_github_get_file_content_tool()`
- `create_github_get_repo_tree_tool()`

In `build_specs()`, register all new specs and handlers.

In `jarvis-rs/core/src/tools/handlers/github.rs`, add handler params and match arms:

- `GetIssueParams { owner, repo, issue_number }`
- `CloseIssueParams { owner, repo, issue_number }`
- `ListIssueCommentsParams { owner, repo, issue_number }`
- `CreatePRParams { owner, repo, title, body, head, base, draft }`
- `CreateBranchParams { owner, repo, branch, from_branch }`
- `GetFileContentParams { owner, repo, path, git_ref }`
- `GetRepoTreeParams { owner, repo, branch }`

### 1H. Update tests

In `jarvis-rs/github/tests/integration_test.rs`, add:

- `test_get_issue` (ignored, needs PAT)
- `test_list_issue_comments` (ignored, needs PAT)
- `test_create_pr` (ignored, needs PAT)
- `test_get_file_content` (ignored, needs PAT)
- `test_get_branch_sha` (ignored, needs PAT)
- `test_get_tree` (ignored, needs PAT)

Unit tests for `FileContent::decoded_content()` in `models.rs` (does NOT need PAT).

### 1I. Run fmt + tests

```bash
cd jarvis-rs && just fmt
cargo test -p jarvis-github
```

---

## FASE 2 -- Issue Analyzer & Planner

### Files to create/modify:

1. **New crate**: `jarvis-rs/issue-resolver/` (or new module in core)
   - `Cargo.toml` with deps on `jarvis-github`, `jarvis-core`
   - `src/lib.rs` - module declarations
   - `src/scanner.rs` - Issue Scanner (polls repos for issues)
   - `src/analyzer.rs` - Issue Analyzer (LLM-powered complexity analysis)
   - `src/context.rs` - Context Builder (repo structure, relevant files)
   - `src/planner.rs` - Implementation Planner

2. **Scanner** (`scanner.rs`):
   - `IssueScanner` struct with config (repos, labels, poll_interval)
   - `scan()` method: calls `list_issues` for each repo with configured labels
   - Filter: skip issues already in progress (tracked in DB)
   - Return: `Vec<CandidateIssue>` sorted by priority

3. **Analyzer** (`analyzer.rs`):
   - `IssueAnalyzer` struct with LLM client
   - `analyze(issue, repo_context) -> IssueAnalysis`:
     ```rust
     pub struct IssueAnalysis {
         pub complexity: Complexity,       // Low, Medium, High
         pub estimated_files: Vec<String>,
         pub approach: String,
         pub tests_needed: Vec<String>,
         pub risks: Vec<String>,
         pub can_auto_resolve: bool,
         pub estimated_tokens: u64,
     }
     ```
   - Uses structured output from LLM
   - Prompt includes: issue body, repo tree, README, similar file patterns

4. **Context Builder** (`context.rs`):
   - `ContextBuilder` struct
   - `build(owner, repo, issue) -> RepoContext`:
     ```rust
     pub struct RepoContext {
         pub language: String,
         pub framework: Option<String>,
         pub test_framework: Option<String>,
         pub relevant_files: Vec<(String, String)>, // (path, content)
         pub tree_structure: String,
         pub readme: Option<String>,
         pub patterns: Vec<CodePattern>,
     }
     ```
   - Uses `get_repo_tree` + `get_file_content` to understand the project
   - Detects language/framework from files (package.json, Cargo.toml, etc.)

5. **Safety Integration**:
   - Wire into existing `SafetyClassifier` from `jarvis-rs/core/src/safety/`
   - Low risk -> auto-execute
   - Medium risk -> execute + notify
   - High risk -> require Telegram approval

---

## FASE 3 -- Code Implementation Engine

### Key components:

1. **New module**: `jarvis-rs/issue-resolver/src/engine.rs`
   - `ImplementationEngine` struct
   - Integrates with existing `AgentLoop` / agentic bridge
   - Custom tool set for code implementation:
     - `read_file`, `write_file`, `search_code`, `run_command`, `git_commit`, `git_diff`

2. **Implementation Flow**:

   ```
   receive CandidateIssue + IssueAnalysis + RepoContext
     -> clone repo locally (or use cached clone)
     -> create branch: jarvis/issue-{number}-{slug}
     -> build implementation prompt
     -> execute AgentLoop with tools
     -> validate: tests pass, build succeeds, lint clean
     -> if validation fails, iterate (max 20 rounds)
     -> return ImplementationResult
   ```

3. **ImplementationResult**:

   ```rust
   pub struct ImplementationResult {
       pub success: bool,
       pub branch_name: String,
       pub commits: Vec<String>,
       pub files_changed: Vec<String>,
       pub tests_added: Vec<String>,
       pub test_results: TestResults,
       pub build_result: BuildResult,
       pub error: Option<String>,
   }
   ```

4. **Quality Gates** (checked after each implementation attempt):
   - All existing tests must pass
   - New tests should be created for the feature
   - Build must compile without errors
   - No changes to files outside the expected scope
   - Lint/format passes

---

## FASE 4 -- PR Creation & Feedback Loop

1. **PR Creator** (`jarvis-rs/issue-resolver/src/pr.rs`):
   - Push branch to remote
   - Create PR via `create_pr` API
   - Formatted body with: summary, changes, approach, tests, validation checklist
   - Link to original issue with "Closes #N"

2. **CI Monitor** (`jarvis-rs/issue-resolver/src/ci.rs`):
   - After PR creation, poll check runs
   - If CI fails -> analyze logs -> attempt auto-fix (max 3 retries)
   - Reuse babysit-pr logic where applicable

3. **Review Responder** (`jarvis-rs/issue-resolver/src/review.rs`):
   - Listen for PR review comments
   - Implement requested changes
   - Push additional commits

---

## FASE 5 -- Daemon Integration & Operations

1. **Pipeline Registration**:
   - Add `IssueResolver` variant to `PipelineStrategy` enum
   - Implement `PipelineRunner` trait
   - Schedule via existing cron system

2. **Configuration** (in `config.toml`):

   ```toml
   [issue_resolver]
   enabled = true
   poll_interval = "30m"
   repos = ["*"]
   labels = ["jarvis-auto", "enhancement"]
   max_complexity = "medium"
   auto_approve = true
   max_concurrent = 2
   max_tokens_per_run = 100000
   ```

3. **Database**:

   ```sql
   CREATE TABLE issue_resolver_runs (
     id INTEGER PRIMARY KEY,
     repo TEXT NOT NULL,
     issue_number INTEGER NOT NULL,
     status TEXT NOT NULL,
     branch_name TEXT,
     pr_url TEXT,
     pr_number INTEGER,
     analysis TEXT,  -- JSON
     started_at INTEGER,
     completed_at INTEGER,
     error_message TEXT,
     llm_tokens_used INTEGER,
     attempts INTEGER DEFAULT 0
   );

   CREATE TABLE issue_resolver_logs (
     id INTEGER PRIMARY KEY,
     run_id INTEGER REFERENCES issue_resolver_runs(id),
     step TEXT NOT NULL,
     message TEXT NOT NULL,
     timestamp INTEGER NOT NULL
   );
   ```

4. **CLI Commands**:

   ```
   jarvis issue-resolver status
   jarvis issue-resolver list
   jarvis issue-resolver run <owner/repo> <issue_number>
   jarvis issue-resolver config
   jarvis issue-resolver history
   ```

5. **Notifications** (Telegram):
   - Issue picked up for resolution
   - PR created (with link)
   - Resolution failed (with error summary)
   - Daily summary

---

## Implementation Order

```
Fase 1 (GitHub API)  ->  independently testable
Fase 2 (Analyzer)    ->  independently testable
Fase 3 (Engine)      ->  independently testable
Fase 4 (PR/CI)       ->  requires Fase 1+3
Fase 5 (Daemon)      ->  requires all above
```

## Risks & Mitigations

| Risk                         | Mitigation                                        |
| ---------------------------- | ------------------------------------------------- |
| LLM generates incorrect code | Quality gates + iteration limits + tests          |
| High token costs             | Token tracking + configurable limits per run      |
| Destructive changes          | Safety classifier + sandbox + isolated branches   |
| Infinite loops               | Timeouts + max iterations + circuit breaker       |
| GitHub credentials           | Minimal-permission PAT token                      |
| Large repos                  | Tree truncation handling + selective file reading |

## Dependencies to Add

- `base64` crate in `jarvis-github/Cargo.toml` (for FileContent decoding)
- No other new external deps needed -- reuses existing jarvis-core infrastructure
