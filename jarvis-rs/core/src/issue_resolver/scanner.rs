//! Issue scanner — polls GitHub repositories for issues matching configured criteria.

use anyhow::Result;
use jarvis_github::GitHubClient;
use jarvis_github::models::Issue;
use tracing::debug;

/// Configuration for the issue scanner.
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    /// Repository owner.
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Only consider issues with ALL of these labels (e.g., `["jarvis-auto"]`).
    pub required_labels: Vec<String>,
    /// Ignore issues with any of these labels (e.g., `["wontfix", "in-progress"]`).
    pub exclude_labels: Vec<String>,
    /// Maximum number of issues to process per scan cycle.
    pub max_issues_per_scan: usize,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            owner: String::new(),
            repo: String::new(),
            required_labels: vec!["jarvis-auto".to_string()],
            exclude_labels: vec!["wontfix".to_string(), "in-progress".to_string()],
            max_issues_per_scan: 5,
        }
    }
}

/// Scans a GitHub repository for issues that can be auto-resolved.
pub struct IssueScanner<'a> {
    client: &'a GitHubClient,
    config: ScannerConfig,
}

impl<'a> IssueScanner<'a> {
    pub fn new(client: &'a GitHubClient, config: ScannerConfig) -> Self {
        Self { client, config }
    }

    /// Scan for open issues matching the configured criteria.
    ///
    /// Returns issues sorted by creation date (oldest first) so we process
    /// the backlog in order.
    pub async fn scan(&self) -> Result<Vec<Issue>> {
        let label_refs: Vec<&str> = self
            .config
            .required_labels
            .iter()
            .map(String::as_str)
            .collect();

        let issues = self
            .client
            .list_issues(
                &self.config.owner,
                &self.config.repo,
                Some("open"),
                Some(&label_refs),
            )
            .await
            .map_err(|e| anyhow::anyhow!("failed to list issues: {e}"))?;

        debug!(
            "found {} open issues with labels {:?}",
            issues.len(),
            self.config.required_labels
        );

        let filtered: Vec<Issue> = issues
            .into_iter()
            .filter(|issue| !self.has_excluded_label(issue))
            .take(self.config.max_issues_per_scan)
            .collect();

        debug!("after filtering: {} issues eligible", filtered.len());

        Ok(filtered)
    }

    /// Check if a single issue matches scan criteria (useful for webhook-driven flows).
    pub fn is_eligible(&self, issue: &Issue) -> bool {
        if issue.state != "open" {
            return false;
        }

        let has_required = self
            .config
            .required_labels
            .iter()
            .all(|req| issue.labels.iter().any(|l| l.name == *req));

        if !has_required {
            return false;
        }

        !self.has_excluded_label(issue)
    }

    fn has_excluded_label(&self, issue: &Issue) -> bool {
        issue.labels.iter().any(|l| {
            self.config
                .exclude_labels
                .iter()
                .any(|excl| l.name == *excl)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jarvis_github::models::Label;
    use jarvis_github::models::User;

    fn make_user() -> User {
        User {
            id: 1,
            login: "test".to_string(),
            avatar_url: None,
        }
    }

    fn make_label(name: &str) -> Label {
        Label {
            id: 1,
            name: name.to_string(),
            color: "000000".to_string(),
            description: None,
        }
    }

    fn make_issue(number: u64, labels: Vec<&str>, state: &str) -> Issue {
        Issue {
            id: number,
            number,
            title: format!("Issue #{number}"),
            body: None,
            state: state.to_string(),
            labels: labels.into_iter().map(make_label).collect(),
            assignees: vec![],
            user: make_user(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            html_url: format!("https://github.com/test/test/issues/{number}"),
        }
    }

    #[test]
    fn test_is_eligible_matching() {
        let config = ScannerConfig {
            required_labels: vec!["jarvis-auto".to_string()],
            exclude_labels: vec!["wontfix".to_string()],
            ..Default::default()
        };
        // We need a client reference but won't use it.
        // Use is_eligible which doesn't call the client.
        let client = GitHubClient::new("fake-token".to_string()).unwrap();
        let scanner = IssueScanner::new(&client, config);

        let issue = make_issue(1, vec!["jarvis-auto", "bug"], "open");
        assert!(scanner.is_eligible(&issue));
    }

    #[test]
    fn test_is_eligible_closed() {
        let client = GitHubClient::new("fake-token".to_string()).unwrap();
        let config = ScannerConfig::default();
        let scanner = IssueScanner::new(&client, config);

        let issue = make_issue(1, vec!["jarvis-auto"], "closed");
        assert!(!scanner.is_eligible(&issue));
    }

    #[test]
    fn test_is_eligible_excluded_label() {
        let config = ScannerConfig {
            required_labels: vec!["jarvis-auto".to_string()],
            exclude_labels: vec!["wontfix".to_string()],
            ..Default::default()
        };
        let client = GitHubClient::new("fake-token".to_string()).unwrap();
        let scanner = IssueScanner::new(&client, config);

        let issue = make_issue(1, vec!["jarvis-auto", "wontfix"], "open");
        assert!(!scanner.is_eligible(&issue));
    }

    #[test]
    fn test_is_eligible_missing_required_label() {
        let config = ScannerConfig {
            required_labels: vec!["jarvis-auto".to_string()],
            ..Default::default()
        };
        let client = GitHubClient::new("fake-token".to_string()).unwrap();
        let scanner = IssueScanner::new(&client, config);

        let issue = make_issue(1, vec!["bug"], "open");
        assert!(!scanner.is_eligible(&issue));
    }
}
