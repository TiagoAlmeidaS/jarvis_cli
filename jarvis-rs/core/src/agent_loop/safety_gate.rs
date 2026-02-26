//! Safety gate for the agentic loop tool executor.
//!
//! Wraps any [`AgentToolExecutor`] with a [`SafetyClassifier`] to intercept
//! tool calls, assess their risk, and block or flag unsafe operations before
//! they are executed.

use std::sync::Arc;

use anyhow::Result;

use super::AgentToolExecutor;
use super::ToolCallRequest;
use super::ToolCallResult;
use super::events::AgentEvent;
use crate::safety::ProposedAction;
use crate::safety::RiskLevel;
use crate::safety::SafetyClassifier;

/// Wraps an [`AgentToolExecutor`] to apply safety classification before each
/// tool execution.
///
/// - `RiskLevel::Low` tools are executed immediately.
/// - `RiskLevel::Medium` tools are executed but an event is emitted.
/// - `RiskLevel::High` and `RiskLevel::Critical` tools are blocked and an
///   error result is returned to the LLM.
pub struct SafeToolExecutor<T: AgentToolExecutor> {
    inner: Arc<T>,
    classifier: Arc<dyn SafetyClassifier>,
    /// Optional callback for safety events (reuses the `on_event` pattern).
    on_event: Option<Arc<dyn Fn(AgentEvent) + Send + Sync>>,
}

impl<T: AgentToolExecutor> SafeToolExecutor<T> {
    /// Creates a new safety-gated executor wrapping `inner`.
    pub fn new(inner: Arc<T>, classifier: Arc<dyn SafetyClassifier>) -> Self {
        Self {
            inner,
            classifier,
            on_event: None,
        }
    }

    /// Attaches an event callback for safety-specific events.
    pub fn with_event_callback(mut self, callback: Arc<dyn Fn(AgentEvent) + Send + Sync>) -> Self {
        self.on_event = Some(callback);
        self
    }

    /// Maps a tool call request to a [`ProposedAction`] for safety assessment.
    fn to_proposed_action(call: &ToolCallRequest) -> ProposedAction {
        let (action_type, files, category) = match call.name.as_str() {
            "shell" => {
                let cmd = call.arguments["command"].as_str().unwrap_or("").to_string();
                let category = classify_shell_command(&cmd);
                ("shell_command".to_string(), vec![], Some(category))
            }
            "write_new_file" => {
                let path = call.arguments["file_path"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let category = classify_file_path(&path);
                ("write_file".to_string(), vec![path], Some(category))
            }
            "read_file" => {
                let path = call.arguments["file_path"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                (
                    "read_file".to_string(),
                    vec![path],
                    Some("read_only".to_string()),
                )
            }
            "list_directory" | "grep_search" => (
                "read_only_operation".to_string(),
                vec![],
                Some("read_only".to_string()),
            ),
            other => (other.to_string(), vec![], None),
        };

        let change = format!(
            "Tool '{}' with arguments: {}",
            call.name,
            call.arguments
                .to_string()
                .chars()
                .take(200)
                .collect::<String>()
        );

        ProposedAction {
            action_type,
            files,
            change,
            impact: format!("Tool execution: {}", call.name),
            category,
        }
    }

    fn emit_event(&self, event: AgentEvent) {
        if let Some(cb) = &self.on_event {
            cb(event);
        }
    }
}

#[async_trait::async_trait]
impl<T: AgentToolExecutor> AgentToolExecutor for SafeToolExecutor<T> {
    fn tool_schemas(&self) -> Vec<serde_json::Value> {
        self.inner.tool_schemas()
    }

    async fn execute(&self, call: &ToolCallRequest) -> Result<ToolCallResult> {
        let action = Self::to_proposed_action(call);

        // Assess safety.
        let assessment = self.classifier.assess_action(&action).await?;

        match assessment.risk_level {
            RiskLevel::Low => {
                // Safe — execute directly.
                self.inner.execute(call).await
            }
            RiskLevel::Medium => {
                // Acceptable risk — execute but emit a warning event.
                self.emit_event(AgentEvent::SafetyWarning {
                    tool_name: call.name.clone(),
                    risk_level: "medium".to_string(),
                    reasoning: assessment.reasoning.clone(),
                });
                self.inner.execute(call).await
            }
            RiskLevel::High | RiskLevel::Critical => {
                // Blocked — return an error to the LLM so it can adjust.
                self.emit_event(AgentEvent::SafetyBlocked {
                    tool_name: call.name.clone(),
                    risk_level: format!("{:?}", assessment.risk_level),
                    reasoning: assessment.reasoning.clone(),
                });
                Ok(ToolCallResult {
                    call_id: call.call_id.clone(),
                    name: call.name.clone(),
                    output: format!(
                        "SAFETY BLOCKED: This action was classified as {:?} risk. \
                         Reason: {}. Please choose a safer approach or ask the user for approval.",
                        assessment.risk_level, assessment.reasoning
                    ),
                    is_error: true,
                })
            }
        }
    }
}

/// Classifies a shell command into a risk category.
fn classify_shell_command(cmd: &str) -> String {
    let cmd_lower = cmd.to_lowercase();

    // Destructive patterns
    let destructive = [
        "rm -rf",
        "rm -r",
        "rmdir",
        "del /s",
        "format ",
        "drop table",
        "drop database",
        "truncate",
        "dd if=",
        "mkfs",
        "> /dev/",
    ];
    for pattern in &destructive {
        if cmd_lower.contains(pattern) {
            return "database".to_string(); // maps to Critical risk
        }
    }

    // Production-modifying patterns
    let production = [
        "deploy",
        "push",
        "publish",
        "release",
        "systemctl",
        "service ",
        "docker push",
        "kubectl apply",
        "terraform apply",
    ];
    for pattern in &production {
        if cmd_lower.contains(pattern) {
            return "production_code".to_string(); // maps to High risk
        }
    }

    // Config-modifying patterns
    let config = ["chmod", "chown", "iptables", "ufw ", "sysctl", "registry "];
    for pattern in &config {
        if cmd_lower.contains(pattern) {
            return "config_file".to_string(); // maps to High risk
        }
    }

    // Safe patterns (read-only, build, test)
    let safe = [
        "ls",
        "cat ",
        "echo ",
        "grep ",
        "rg ",
        "find ",
        "cargo test",
        "cargo check",
        "cargo build",
        "cargo fmt",
        "npm test",
        "npm run",
        "pytest",
        "go test",
        "git status",
        "git log",
        "git diff",
        "pwd",
        "whoami",
        "uname",
        "date",
    ];
    for pattern in &safe {
        if cmd_lower.starts_with(pattern) || cmd_lower.contains(pattern) {
            return "test_file".to_string(); // maps to Low risk
        }
    }

    // Default: medium risk for unknown commands
    "unknown".to_string()
}

/// Classifies a file path into a risk category.
fn classify_file_path(path: &str) -> String {
    let path_lower = path.to_lowercase();

    if path_lower.contains("test") || path_lower.contains("spec") || path_lower.contains("_test") {
        "test_file".to_string()
    } else if path_lower.ends_with(".md")
        || path_lower.ends_with(".txt")
        || path_lower.ends_with(".rst")
    {
        "documentation".to_string()
    } else if path_lower.ends_with(".toml")
        || path_lower.ends_with(".yaml")
        || path_lower.ends_with(".yml")
        || path_lower.ends_with(".json")
        || path_lower.ends_with(".env")
    {
        "config_file".to_string()
    } else if path_lower.ends_with(".rs")
        || path_lower.ends_with(".py")
        || path_lower.ends_with(".js")
        || path_lower.ends_with(".ts")
    {
        "production_code".to_string()
    } else {
        "unknown".to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::RuleBasedSafetyClassifier;
    use pretty_assertions::assert_eq;

    // Simple passthrough executor for testing.
    struct MockExecutor;

    #[async_trait::async_trait]
    impl AgentToolExecutor for MockExecutor {
        fn tool_schemas(&self) -> Vec<serde_json::Value> {
            vec![]
        }

        async fn execute(&self, call: &ToolCallRequest) -> Result<ToolCallResult> {
            Ok(ToolCallResult {
                call_id: call.call_id.clone(),
                name: call.name.clone(),
                output: "mock output".to_string(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn safe_tool_passes_through() {
        let inner = Arc::new(MockExecutor);
        let classifier = Arc::new(RuleBasedSafetyClassifier::default());
        let executor = SafeToolExecutor::new(inner, classifier);

        let call = ToolCallRequest {
            call_id: "c1".to_string(),
            name: "list_directory".to_string(),
            arguments: serde_json::json!({"path": "."}),
        };

        let result = executor.execute(&call).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.output, "mock output");
    }

    #[tokio::test]
    async fn destructive_shell_command_is_blocked() {
        let inner = Arc::new(MockExecutor);
        let classifier = Arc::new(RuleBasedSafetyClassifier::default());
        let executor = SafeToolExecutor::new(inner, classifier);

        let call = ToolCallRequest {
            call_id: "c2".to_string(),
            name: "shell".to_string(),
            arguments: serde_json::json!({"command": "rm -rf /"}),
        };

        let result = executor.execute(&call).await.unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("SAFETY BLOCKED"));
    }

    #[tokio::test]
    async fn safe_shell_command_passes() {
        let inner = Arc::new(MockExecutor);
        let classifier = Arc::new(RuleBasedSafetyClassifier::default());
        let executor = SafeToolExecutor::new(inner, classifier);

        let call = ToolCallRequest {
            call_id: "c3".to_string(),
            name: "shell".to_string(),
            arguments: serde_json::json!({"command": "cargo test"}),
        };

        let result = executor.execute(&call).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.output, "mock output");
    }

    #[tokio::test]
    async fn write_to_production_file_is_blocked() {
        let inner = Arc::new(MockExecutor);
        let classifier = Arc::new(RuleBasedSafetyClassifier::default());
        let executor = SafeToolExecutor::new(inner, classifier);

        let call = ToolCallRequest {
            call_id: "c4".to_string(),
            name: "write_new_file".to_string(),
            arguments: serde_json::json!({
                "file_path": "src/main.rs",
                "content": "fn main() {}"
            }),
        };

        let result = executor.execute(&call).await.unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("SAFETY BLOCKED"));
    }

    #[tokio::test]
    async fn write_to_test_file_passes() {
        let inner = Arc::new(MockExecutor);
        let classifier = Arc::new(RuleBasedSafetyClassifier::default());
        let executor = SafeToolExecutor::new(inner, classifier);

        let call = ToolCallRequest {
            call_id: "c5".to_string(),
            name: "write_new_file".to_string(),
            arguments: serde_json::json!({
                "file_path": "tests/test_foo.rs",
                "content": "#[test] fn test_foo() {}"
            }),
        };

        let result = executor.execute(&call).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.output, "mock output");
    }

    #[test]
    fn classify_shell_commands() {
        assert_eq!(classify_shell_command("rm -rf /tmp"), "database");
        assert_eq!(classify_shell_command("cargo test"), "test_file");
        assert_eq!(
            classify_shell_command("docker push myimage"),
            "production_code"
        );
        assert_eq!(
            classify_shell_command("chmod 777 /etc/passwd"),
            "config_file"
        );
        assert_eq!(classify_shell_command("some_unknown_cmd"), "unknown");
    }

    #[test]
    fn classify_file_paths() {
        assert_eq!(classify_file_path("tests/test_foo.rs"), "test_file");
        assert_eq!(classify_file_path("src/main.rs"), "production_code");
        assert_eq!(classify_file_path("README.md"), "documentation");
        assert_eq!(classify_file_path("config.toml"), "config_file");
        assert_eq!(classify_file_path("data.bin"), "unknown");
    }
}
