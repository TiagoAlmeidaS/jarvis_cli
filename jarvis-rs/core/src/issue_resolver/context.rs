//! Repository context builder for issue analysis.
//!
//! Gathers structural and content information about a repository so the
//! analyzer and planner can make informed decisions.

use anyhow::Result;
use jarvis_github::GitHubClient;
use tracing::debug;
use tracing::warn;

use super::types::RelevantFile;
use super::types::RepoContext;

/// Maximum number of tree entries to include in the summary.
const MAX_TREE_SUMMARY_ENTRIES: usize = 200;

/// Maximum size (bytes) for fetched file content.
const MAX_FILE_CONTENT_BYTES: usize = 50_000;

/// Well-known files that help understand a repository.
const WELL_KNOWN_FILES: &[&str] = &[
    "README.md",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    ".github/CODEOWNERS",
    "CONTRIBUTING.md",
];

/// Builds a [`RepoContext`] by querying the GitHub API.
pub struct ContextBuilder<'a> {
    client: &'a GitHubClient,
    owner: String,
    repo: String,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(client: &'a GitHubClient, owner: &str, repo: &str) -> Self {
        Self {
            client,
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }

    /// Build a complete [`RepoContext`] for use by the analyzer/planner.
    pub async fn build(&self) -> Result<RepoContext> {
        // Fetch repo metadata to get default branch and language.
        let repo_info = self
            .client
            .get_repo(&self.owner, &self.repo)
            .await
            .map_err(|e| anyhow::anyhow!("failed to fetch repo info: {e}"))?;

        let default_branch = repo_info.default_branch.clone();

        // Fetch tree structure (best-effort).
        let tree_summary = self.build_tree_summary(&default_branch).await;

        // Detect language and framework from tree.
        let language = detect_language(&tree_summary);
        let framework = detect_framework(&tree_summary);

        // Fetch well-known files for context.
        let relevant_files = self.fetch_well_known_files(&default_branch).await;

        // Extract README content separately for top-level context.
        let readme = relevant_files
            .iter()
            .find(|f| f.path.eq_ignore_ascii_case("README.md"))
            .and_then(|f| f.content.clone());

        // Detect coding patterns from the tree and README.
        let patterns = detect_patterns(&tree_summary, readme.as_deref());

        Ok(RepoContext {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            language,
            framework,
            relevant_files,
            tree_summary,
            readme,
            patterns,
            default_branch,
        })
    }

    /// Fetch additional files that may be relevant to a specific issue.
    pub async fn fetch_files_for_issue(
        &self,
        branch: &str,
        file_paths: &[String],
    ) -> Vec<RelevantFile> {
        let mut files = Vec::new();
        for path in file_paths {
            match self
                .client
                .get_file_content(&self.owner, &self.repo, path, Some(branch))
                .await
            {
                Ok(fc) => {
                    let content = fc
                        .decoded_content()
                        .filter(|c| c.len() <= MAX_FILE_CONTENT_BYTES);
                    files.push(RelevantFile {
                        path: path.clone(),
                        reason: "estimated to require changes".to_string(),
                        content,
                    });
                }
                Err(e) => {
                    warn!("could not fetch {path}: {e}");
                }
            }
        }
        files
    }

    // ---- internal helpers ----

    async fn build_tree_summary(&self, branch: &str) -> String {
        let sha = match self
            .client
            .get_branch_sha(&self.owner, &self.repo, branch)
            .await
        {
            Ok(sha) => sha,
            Err(e) => {
                warn!("could not get branch SHA for {branch}: {e}");
                return String::new();
            }
        };

        let tree = match self.client.get_tree(&self.owner, &self.repo, &sha).await {
            Ok(t) => t,
            Err(e) => {
                warn!("could not get tree for SHA {sha}: {e}");
                return String::new();
            }
        };

        let mut lines: Vec<String> = tree
            .tree
            .iter()
            .take(MAX_TREE_SUMMARY_ENTRIES)
            .map(|entry| {
                if entry.entry_type == "tree" {
                    format!("{}/", entry.path)
                } else {
                    entry.path.clone()
                }
            })
            .collect();

        if tree.truncated || tree.tree.len() > MAX_TREE_SUMMARY_ENTRIES {
            lines.push(format!(
                "... ({} entries total, truncated)",
                tree.tree.len()
            ));
        }

        lines.join("\n")
    }

    async fn fetch_well_known_files(&self, branch: &str) -> Vec<RelevantFile> {
        let mut files = Vec::new();

        for path in WELL_KNOWN_FILES {
            match self
                .client
                .get_file_content(&self.owner, &self.repo, path, Some(branch))
                .await
            {
                Ok(fc) => {
                    let content = fc
                        .decoded_content()
                        .filter(|c| c.len() <= MAX_FILE_CONTENT_BYTES);
                    debug!("fetched well-known file: {path}");
                    files.push(RelevantFile {
                        path: path.to_string(),
                        reason: "well-known repository file".to_string(),
                        content,
                    });
                }
                Err(_) => {
                    // File does not exist in this repo -- that is fine.
                }
            }
        }

        files
    }
}

/// Detect the primary language from a tree summary.
fn detect_language(tree_summary: &str) -> Option<String> {
    let checks: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust"),
        (".rs", "Rust"),
        ("package.json", "JavaScript/TypeScript"),
        (".ts", "TypeScript"),
        (".py", "Python"),
        ("go.mod", "Go"),
        (".go", "Go"),
        (".java", "Java"),
        (".cs", "C#"),
        (".rb", "Ruby"),
    ];

    for (pattern, lang) in checks {
        if tree_summary.contains(pattern) {
            return Some(lang.to_string());
        }
    }
    None
}

/// Detect the framework from a tree summary.
fn detect_framework(tree_summary: &str) -> Option<String> {
    let checks: &[(&str, &str)] = &[
        ("next.config", "Next.js"),
        ("angular.json", "Angular"),
        ("vue.config", "Vue"),
        ("django", "Django"),
        ("flask", "Flask"),
        ("actix", "Actix"),
        ("axum", "Axum"),
        ("rocket", "Rocket"),
        ("spring", "Spring"),
        ("rails", "Rails"),
    ];

    for (pattern, fw) in checks {
        if tree_summary.to_lowercase().contains(pattern) {
            return Some(fw.to_string());
        }
    }
    None
}

/// Detect coding patterns from tree summary and README.
fn detect_patterns(tree_summary: &str, readme: Option<&str>) -> Vec<String> {
    let mut patterns = Vec::new();

    if tree_summary.contains("tests/") || tree_summary.contains("test/") {
        patterns.push("Has dedicated test directory".to_string());
    }
    if tree_summary.contains("src/") {
        patterns.push("Uses src/ directory layout".to_string());
    }
    if tree_summary.contains(".github/workflows/") {
        patterns.push("Has GitHub Actions CI".to_string());
    }
    if tree_summary.contains("Dockerfile") {
        patterns.push("Uses Docker".to_string());
    }
    if tree_summary.contains(".eslintrc") || tree_summary.contains("eslint.config") {
        patterns.push("Uses ESLint".to_string());
    }
    if tree_summary.contains("clippy") || tree_summary.contains("rustfmt.toml") {
        patterns.push("Uses Rust formatting/linting".to_string());
    }

    if let Some(readme_text) = readme {
        let readme_lower = readme_text.to_lowercase();
        if readme_lower.contains("contributing") {
            patterns.push("Has contribution guidelines".to_string());
        }
        if readme_lower.contains("license") {
            patterns.push("Has license information".to_string());
        }
    }

    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_rust() {
        let tree = "src/main.rs\nCargo.toml\nREADME.md";
        assert_eq!(detect_language(tree), Some("Rust".to_string()));
    }

    #[test]
    fn test_detect_language_python() {
        let tree = "setup.py\nsrc/app.py\ntests/test_app.py";
        assert_eq!(detect_language(tree), Some("Python".to_string()));
    }

    #[test]
    fn test_detect_language_none() {
        let tree = "README.md\nLICENSE";
        assert_eq!(detect_language(tree), None);
    }

    #[test]
    fn test_detect_framework_nextjs() {
        let tree = "next.config.js\npackage.json\npages/index.tsx";
        assert_eq!(detect_framework(tree), Some("Next.js".to_string()));
    }

    #[test]
    fn test_detect_framework_none() {
        let tree = "src/main.rs\nCargo.toml";
        assert_eq!(detect_framework(tree), None);
    }

    #[test]
    fn test_detect_patterns() {
        let tree = "src/lib.rs\ntests/integration.rs\n.github/workflows/ci.yml";
        let patterns = detect_patterns(tree, None);
        assert!(patterns.contains(&"Has dedicated test directory".to_string()));
        assert!(patterns.contains(&"Uses src/ directory layout".to_string()));
        assert!(patterns.contains(&"Has GitHub Actions CI".to_string()));
    }

    #[test]
    fn test_detect_patterns_with_readme() {
        let tree = "src/lib.rs";
        let readme =
            "# My Project\n\n## Contributing\n\nPlease read our guidelines.\n\n## License\n\nMIT";
        let patterns = detect_patterns(tree, Some(readme));
        assert!(patterns.contains(&"Has contribution guidelines".to_string()));
        assert!(patterns.contains(&"Has license information".to_string()));
    }
}
