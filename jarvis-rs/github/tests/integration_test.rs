#![allow(clippy::unwrap_used)]

use jarvis_github::GitHubClient;
use jarvis_github::GitHubError;
use pretty_assertions::assert_eq;

// Note: These are integration tests that require a real GitHub PAT.
// They are skipped by default unless GITHUB_PAT environment variable is set.
// To run: GITHUB_PAT=your_token cargo test --test integration_test

fn get_test_pat() -> Option<String> {
    std::env::var("GITHUB_PAT").ok()
}

#[tokio::test]
#[ignore]
async fn test_create_issue() {
    let pat = get_test_pat().expect("GITHUB_PAT environment variable must be set");
    let client = GitHubClient::new(pat.clone()).unwrap();

    // Use a test repository - replace with your own
    let owner = "octocat";
    let repo = "Hello-World";

    let issue = jarvis_github::issues::create_issue(
        &client,
        owner,
        repo,
        jarvis_github::models::IssueCreateRequest {
            title: "Test Issue from Jarvis".to_string(),
            body: Some("This is a test issue created by Jarvis CLI".to_string()),
            labels: None,
            assignees: None,
        },
    )
    .await;

    match issue {
        Ok(issue) => {
            assert_eq!(issue.title, "Test Issue from Jarvis");
            assert!(issue.number > 0);
        }
        Err(GitHubError::Api { status, message }) if status == 404 => {
            // Repository doesn't exist or is private - this is expected for test
            eprintln!("Repository {owner}/{repo} not found or not accessible: {message}");
        }
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[tokio::test]
#[ignore]
async fn test_list_repositories() {
    let pat = get_test_pat().expect("GITHUB_PAT environment variable must be set");
    let client = GitHubClient::new(pat.clone()).unwrap();

    let repos = jarvis_github::repositories::list_repositories(&client, None).await;

    match repos {
        Ok(repos) => {
            assert!(!repos.is_empty(), "Should list at least one repository");
            // Verify repository structure
            if let Some(repo) = repos.first() {
                assert!(!repo.name.is_empty());
                assert!(!repo.full_name.is_empty());
            }
        }
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[tokio::test]
#[ignore]
async fn test_get_repository() {
    let pat = get_test_pat().expect("GITHUB_PAT environment variable must be set");
    let client = GitHubClient::new(pat.clone()).unwrap();

    // Use a public repository for testing
    let owner = "octocat";
    let repo = "Hello-World";

    let repo_info = jarvis_github::repositories::get_repo(&client, owner, repo).await;

    match repo_info {
        Ok(repo_info) => {
            assert_eq!(repo_info.name, repo);
            assert_eq!(repo_info.full_name, format!("{owner}/{repo}"));
        }
        Err(GitHubError::Api { status, message }) if status == 404 => {
            eprintln!("Repository {owner}/{repo} not found: {message}");
        }
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[tokio::test]
#[ignore]
async fn test_list_issues() {
    let pat = get_test_pat().expect("GITHUB_PAT environment variable must be set");
    let client = GitHubClient::new(pat.clone()).unwrap();

    // Use a public repository with issues
    let owner = "octocat";
    let repo = "Hello-World";

    let issues = jarvis_github::issues::list_issues(&client, owner, repo, None, None).await;

    match issues {
        Ok(issues) => {
            // Repository may or may not have issues
            for issue in issues {
                assert!(issue.number > 0);
                assert!(!issue.title.is_empty());
            }
        }
        Err(GitHubError::Api { status, message }) if status == 404 => {
            eprintln!("Repository {owner}/{repo} not found: {message}");
        }
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[tokio::test]
async fn test_client_with_custom_base_url() {
    let pat = "test_token";
    let custom_url = "https://api.github.com";
    let client = GitHubClient::with_base_url(pat.to_string(), custom_url.to_string()).unwrap();
    assert_eq!(client.base_url(), custom_url);
}

#[tokio::test]
async fn test_client_with_default_base_url() {
    let pat = "test_token";
    let client = GitHubClient::new(pat.to_string()).unwrap();
    assert_eq!(client.base_url(), "https://api.github.com");
}
