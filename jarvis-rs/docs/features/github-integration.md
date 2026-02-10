# GitHub Integration

Jarvis CLI provides native integration with GitHub API, allowing you to interact with repositories, issues, and pull requests directly from the command line.

## Overview

The GitHub integration enables Jarvis to:
- Create and manage GitHub issues
- Comment on pull requests
- List and search repositories
- Get repository clone URLs
- List issues with filtering options

All operations use secure Personal Access Token (PAT) authentication stored in Jarvis secrets.

## Configuration

### Setting up GitHub PAT

You can configure the GitHub PAT using one of the following methods (in priority order):

1. **Environment Variable** (recommended for CI/CD and temporary setups):
   ```bash
   export GITHUB_PAT=<your-token>
   # or
   export JARVIS_GITHUB_PAT=<your-token>
   ```

2. **Jarvis Secrets** (recommended for persistent local storage):
   ```bash
   jarvis secrets set GITHUB_PAT <your-token>
   ```

**Creating a GitHub Personal Access Token:**
- Go to GitHub Settings → Developer settings → Personal access tokens → Tokens (classic)
- Generate a new token with appropriate scopes:
  - `repo` - Full control of private repositories
  - `public_repo` - Access public repositories
  - `read:org` - Read organization membership (if needed)

### GitHub Configuration

#### Environment Variables

You can configure GitHub settings using environment variables:

- `GITHUB_PAT` or `JARVIS_GITHUB_PAT` - GitHub Personal Access Token
- `GITHUB_API_BASE_URL` or `JARVIS_GITHUB_API_BASE_URL` - GitHub API base URL (for GitHub Enterprise Server)

**Example:**
```bash
export GITHUB_PAT=ghp_your_token_here
export GITHUB_API_BASE_URL=https://github.example.com/api/v3  # For GitHub Enterprise
```

#### Configuration File

Alternatively, add GitHub configuration to your `config.toml`:

```toml
[github]
# Name of the secret storing the GitHub PAT (default: "GITHUB_PAT")
# Only used if environment variables are not set
pat_secret_name = "GITHUB_PAT"

# Base URL for GitHub API (default: "https://api.github.com")
# Use this for GitHub Enterprise Server:
# api_base_url = "https://github.example.com/api/v3"
api_base_url = "https://api.github.com"
```

**Priority Order:**
1. Environment variables (`GITHUB_PAT` or `JARVIS_GITHUB_PAT`)
2. Secrets manager (using `pat_secret_name` from config)
3. Configuration file defaults

## Available Tools

### github_create_issue

Create a new issue in a GitHub repository.

**Parameters:**
- `owner` (required): Repository owner (username or organization)
- `repo` (required): Repository name
- `title` (required): Issue title
- `body` (optional): Issue description/body
- `labels` (optional): Array of label names to apply
- `assignees` (optional): Array of GitHub usernames to assign

**Example:**
```bash
jarvis "Create an issue in my-repo/example titled 'Fix bug' with body 'Description of the bug'"
```

### github_comment_pr

Add a comment to a pull request.

**Parameters:**
- `owner` (required): Repository owner
- `repo` (required): Repository name
- `pr_number` (required): Pull request number
- `comment` (required): Comment body (supports Markdown)

**Example:**
```bash
jarvis "Comment on PR #123 in my-repo/example saying 'Looks good!'"
```

### github_list_repos

List repositories for a user or organization.

**Parameters:**
- `username` (optional): GitHub username or organization. If omitted, lists repositories for the authenticated user.

**Example:**
```bash
jarvis "List all repositories for octocat"
```

### github_clone_repo

Get the clone URL for a repository.

**Parameters:**
- `owner` (required): Repository owner
- `repo` (required): Repository name
- `use_ssh` (optional): If true, return SSH URL; otherwise return HTTPS URL (default: false)

**Example:**
```bash
jarvis "Get the clone URL for my-repo/example"
```

### github_list_issues

List issues in a repository with optional filtering.

**Parameters:**
- `owner` (required): Repository owner
- `repo` (required): Repository name
- `state` (optional): Filter by state: "open", "closed", or "all" (default: "open")
- `labels` (optional): Array of label names to filter by

**Example:**
```bash
jarvis "List all open issues in my-repo/example with label 'bug'"
```

## Usage Examples

### Creating an Issue

```bash
jarvis "Create a new issue in my-org/my-repo titled 'Add feature X' with body 'This feature should...' and assign it to @username"
```

### Commenting on a PR

```bash
jarvis "Add a comment to PR #42 in my-org/my-repo saying 'This looks great! Just one small suggestion...'"
```

### Listing Repositories

```bash
jarvis "Show me all repositories for the my-org organization"
```

### Getting Clone URL

```bash
jarvis "What's the SSH clone URL for my-org/my-repo?"
```

### Listing Issues

```bash
jarvis "Show me all closed issues in my-org/my-repo that have the 'bug' label"
```

## Error Handling

The GitHub integration handles common errors gracefully:

- **Authentication errors**: Clear message indicating PAT is missing or invalid
- **Rate limiting**: Information about when the rate limit resets
- **API errors**: Detailed error messages with status codes
- **Repository not found**: Helpful error messages for 404 errors

## Security Considerations

- PATs are stored securely using Jarvis secrets management
- PATs are never exposed in logs or error messages
- Use the minimum required scopes for your use case
- Regularly rotate PATs for security

## GitHub Enterprise Server

To use with GitHub Enterprise Server, configure the `api_base_url`:

```toml
[github]
api_base_url = "https://github.example.com/api/v3"
pat_secret_name = "GITHUB_PAT"
```

## Troubleshooting

### "GitHub PAT not found" error

Make sure you've set the PAT using one of these methods:

**Option 1: Environment Variable** (recommended for CI/CD):
```bash
export GITHUB_PAT=<your-token>
```

**Option 2: Jarvis Secrets** (recommended for local development):
```bash
jarvis secrets set GITHUB_PAT <your-token>
```

The system checks environment variables first, then falls back to secrets manager.

### "Authentication failed" error

- Verify your PAT is valid and not expired
- Check that the PAT has the required scopes
- For Enterprise Server, ensure the API base URL is correct

### Rate limiting

GitHub API has rate limits. If you hit the limit:
- Wait for the reset time indicated in the error message
- Consider using a PAT with higher rate limits
- Implement retry logic in your workflows

## See Also

- [Configuration Documentation](../config.md)
- [Secrets Management](../secrets.md)
- [GitHub API Documentation](https://docs.github.com/en/rest)
