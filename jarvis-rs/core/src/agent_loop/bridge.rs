//! Bridge between the AgentLoop and the TUI / CLI.
//!
//! This module provides concrete implementations of [`AgentLlmClient`] and
//! [`AgentToolExecutor`] that work with standard OpenAI-compatible APIs
//! and simple local tool execution (shell, read_file, list_directory,
//! grep_search, write_new_file).
//!
//! These implementations enable models without native function calling
//! to use tools via text-based tool calling, using the [`super::AgentLoop`]
//! Think → Execute → Observe cycle.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{
    AgentLlmClient, AgentMessage, AgentToolExecutor, ThinkResponse, ToolCallRequest, ToolCallResult,
};
use crate::tools::text_tool_calling::{
    build_tool_prompt_injection, default_text_tool_specs, parse_tool_calls_from_text, TextToolSpec,
};

// Re-export for consumers outside the core crate.
pub use crate::tools::text_tool_calling::{
    default_text_tool_specs as default_tool_specs, TextToolSpec as ToolSpec, ToolCallingMode,
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the bridge LLM client.
#[derive(Debug, Clone, Deserialize)]
pub struct BridgeLlmConfig {
    /// OpenAI-compatible API base URL.
    pub base_url: String,
    /// API key (can be empty for local models).
    #[serde(default)]
    pub api_key: String,
    /// Model identifier.
    pub model: String,
    /// Temperature (0.0 - 2.0).
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Max tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout_sec")]
    pub timeout_sec: u64,
}

fn default_temperature() -> f64 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_timeout_sec() -> u64 {
    120
}

impl Default for BridgeLlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434/v1".to_string(),
            api_key: String::new(),
            model: "mistral".to_string(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            timeout_sec: default_timeout_sec(),
        }
    }
}

// ---------------------------------------------------------------------------
// LLM Client implementation
// ---------------------------------------------------------------------------

/// An [`AgentLlmClient`] backed by an OpenAI-compatible API.
///
/// When the model doesn't support native function calling, tool descriptions
/// are injected into the system prompt and tool calls are parsed from the
/// model's text output.
pub struct BridgeLlmClient {
    http: reqwest::Client,
    config: BridgeLlmConfig,
    /// Tool prompt to inject into the system message.
    tool_prompt_suffix: String,
}

impl BridgeLlmClient {
    pub fn new(config: BridgeLlmConfig, tool_specs: &[TextToolSpec]) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_sec))
            .build()
            .context("failed to build HTTP client")?;

        let tool_prompt_suffix = build_tool_prompt_injection(tool_specs);

        Ok(Self {
            http,
            config,
            tool_prompt_suffix,
        })
    }

    /// Convert our AgentMessage list into OpenAI chat messages.
    fn build_openai_messages(&self, messages: &[AgentMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|msg| match msg {
                AgentMessage::System(text) => {
                    // Inject tool descriptions at the end of the system prompt.
                    let enriched = format!("{text}{}", self.tool_prompt_suffix);
                    serde_json::json!({ "role": "system", "content": enriched })
                }
                AgentMessage::User(text) => {
                    serde_json::json!({ "role": "user", "content": text })
                }
                AgentMessage::Assistant(content) => {
                    if content.tool_calls.is_empty() {
                        serde_json::json!({ "role": "assistant", "content": content.text })
                    } else {
                        // For text-based models, include the raw text that contained
                        // tool calls (the model needs context of what it said).
                        let mut text = content.text.clone();
                        for tc in &content.tool_calls {
                            text.push_str(&format!(
                                "\n```tool_call\n{}\n```\n",
                                serde_json::json!({
                                    "name": tc.name,
                                    "arguments": tc.arguments,
                                })
                            ));
                        }
                        serde_json::json!({ "role": "assistant", "content": text })
                    }
                }
                AgentMessage::ToolResult { call_id, output } => {
                    // Text-based models see tool results as user messages.
                    let formatted = crate::tools::text_tool_calling::format_tool_result_for_text(
                        "tool", call_id, output,
                    );
                    serde_json::json!({ "role": "user", "content": formatted })
                }
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl AgentLlmClient for BridgeLlmClient {
    async fn think(
        &self,
        messages: &[AgentMessage],
        _tools: &[serde_json::Value],
    ) -> Result<ThinkResponse> {
        let openai_messages = self.build_openai_messages(messages);

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": openai_messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
        });

        let mut req = self.http.post(&url).json(&body);
        if !self.config.api_key.is_empty() {
            req = req.bearer_auth(&self.config.api_key);
        }

        let resp = req.send().await.context("LLM request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM returned {status}: {text}");
        }

        let json: serde_json::Value = resp.json().await.context("failed to parse LLM response")?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Parse text-based tool calls from the response.
        let (remaining_text, parsed_calls) = parse_tool_calls_from_text(&content);

        let tool_calls: Vec<ToolCallRequest> = parsed_calls
            .into_iter()
            .map(|pc| ToolCallRequest {
                call_id: pc.call_id,
                name: pc.name,
                arguments: pc.arguments,
            })
            .collect();

        Ok(ThinkResponse {
            content: remaining_text,
            tool_calls,
        })
    }
}

// ---------------------------------------------------------------------------
// Tool Executor implementation
// ---------------------------------------------------------------------------

/// Configuration for the local tool executor.
#[derive(Debug, Clone)]
pub struct BridgeToolConfig {
    /// Working directory for shell commands.
    pub working_dir: PathBuf,
    /// Maximum shell command execution time.
    pub shell_timeout: Duration,
    /// Whether shell commands require approval (for safety).
    pub require_approval: bool,
}

impl Default for BridgeToolConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            shell_timeout: Duration::from_secs(30),
            require_approval: false,
        }
    }
}

/// A local tool executor that handles shell commands, file reads, directory
/// listings, grep searches, and file writes.
///
/// This is intentionally lightweight and does NOT go through the full
/// TUI approval flow. For production use in the TUI, the approval
/// callback should be provided.
pub struct BridgeToolExecutor {
    config: BridgeToolConfig,
    tool_specs: Vec<TextToolSpec>,
}

impl BridgeToolExecutor {
    pub fn new(config: BridgeToolConfig) -> Self {
        Self {
            config,
            tool_specs: default_text_tool_specs(),
        }
    }

    pub fn with_tools(mut self, specs: Vec<TextToolSpec>) -> Self {
        self.tool_specs = specs;
        self
    }

    async fn exec_shell(&self, args: &serde_json::Value) -> Result<String> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'command' argument"))?;
        let workdir = args["workdir"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.working_dir.clone());

        let output = tokio::process::Command::new(shell_program())
            .arg(shell_arg())
            .arg(command)
            .current_dir(&workdir)
            .output()
            .await
            .context("failed to execute shell command")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(format!(
            "exit_code: {exit_code}\nstdout:\n{stdout}\nstderr:\n{stderr}"
        ))
    }

    async fn exec_read_file(&self, args: &serde_json::Value) -> Result<String> {
        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'file_path' argument"))?;
        let offset = args["offset"].as_u64().unwrap_or(1) as usize;
        let limit = args["limit"].as_u64().unwrap_or(2000) as usize;

        let content = tokio::fs::read_to_string(file_path)
            .await
            .with_context(|| format!("failed to read file: {file_path}"))?;

        let lines: Vec<&str> = content.lines().collect();
        let start = (offset.saturating_sub(1)).min(lines.len());
        let end = (start + limit).min(lines.len());

        let selected: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>6}|{line}", start + i + 1))
            .collect();

        Ok(selected.join("\n"))
    }

    async fn exec_list_directory(&self, args: &serde_json::Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        let mut entries = tokio::fs::read_dir(path)
            .await
            .with_context(|| format!("failed to read directory: {path}"))?;

        let mut items = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let ft = entry.file_type().await?;
            let prefix = if ft.is_dir() { "d " } else { "f " };
            items.push(format!("{prefix}{name}"));
        }
        items.sort();

        Ok(items.join("\n"))
    }

    async fn exec_grep_search(&self, args: &serde_json::Value) -> Result<String> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'pattern' argument"))?;
        let path = args["path"].as_str().unwrap_or(".");
        let include = args["include"].as_str();

        let mut cmd = tokio::process::Command::new("rg");
        cmd.arg("--max-count=50")
            .arg("--line-number")
            .arg(pattern)
            .arg(path);

        if let Some(glob) = include {
            cmd.arg("--glob").arg(glob);
        }

        let output = cmd.output().await.context("failed to execute rg")?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            Ok(stdout.to_string())
        }
    }

    async fn exec_write_file(&self, args: &serde_json::Value) -> Result<String> {
        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'file_path' argument"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;

        // Ensure parent directory exists.
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(file_path, content)
            .await
            .with_context(|| format!("failed to write file: {file_path}"))?;

        Ok(format!(
            "File written: {file_path} ({} bytes)",
            content.len()
        ))
    }
}

/// Get the platform shell program.
fn shell_program() -> &'static str {
    if cfg!(windows) {
        "cmd"
    } else {
        "sh"
    }
}

/// Get the shell argument flag.
fn shell_arg() -> &'static str {
    if cfg!(windows) {
        "/C"
    } else {
        "-c"
    }
}

#[async_trait::async_trait]
impl AgentToolExecutor for BridgeToolExecutor {
    fn tool_schemas(&self) -> Vec<serde_json::Value> {
        self.tool_specs
            .iter()
            .map(|spec| {
                let params: serde_json::Value = serde_json::json!({
                    "type": "object",
                    "properties": spec.parameters.iter().map(|p| {
                        (p.name.clone(), serde_json::json!({
                            "type": p.param_type,
                            "description": p.description,
                        }))
                    }).collect::<serde_json::Map<String, serde_json::Value>>(),
                    "required": spec.parameters.iter()
                        .filter(|p| p.required)
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>(),
                });

                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": spec.name,
                        "description": spec.description,
                        "parameters": params,
                    }
                })
            })
            .collect()
    }

    async fn execute(&self, call: &ToolCallRequest) -> Result<ToolCallResult> {
        let output = match call.name.as_str() {
            "shell" => self.exec_shell(&call.arguments).await,
            "read_file" => self.exec_read_file(&call.arguments).await,
            "list_directory" => self.exec_list_directory(&call.arguments).await,
            "grep_search" => self.exec_grep_search(&call.arguments).await,
            "write_new_file" => self.exec_write_file(&call.arguments).await,
            other => Err(anyhow::anyhow!("unknown tool: {other}")),
        };

        match output {
            Ok(out) => Ok(ToolCallResult {
                call_id: call.call_id.clone(),
                name: call.name.clone(),
                output: out,
                is_error: false,
            }),
            Err(e) => Ok(ToolCallResult {
                call_id: call.call_id.clone(),
                name: call.name.clone(),
                output: format!("Error: {e:#}"),
                is_error: true,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::sync::Arc;

    #[test]
    fn bridge_llm_config_defaults() {
        let config = BridgeLlmConfig::default();
        assert_eq!(config.model, "mistral");
        assert!((config.temperature - 0.7).abs() < 0.01);
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout_sec, 120);
    }

    #[test]
    fn bridge_llm_config_deserialize() {
        let config: BridgeLlmConfig = serde_json::from_value(serde_json::json!({
            "base_url": "http://localhost:11434/v1",
            "model": "llama3.1",
            "temperature": 0.3,
            "max_tokens": 2048,
        }))
        .expect("parse");
        assert_eq!(config.model, "llama3.1");
        assert!((config.temperature - 0.3).abs() < 0.01);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn bridge_tool_config_defaults() {
        let config = BridgeToolConfig::default();
        assert_eq!(config.shell_timeout, Duration::from_secs(30));
        assert!(!config.require_approval);
    }

    #[test]
    fn bridge_tool_executor_schemas() {
        let executor = BridgeToolExecutor::new(BridgeToolConfig::default());
        let schemas = executor.tool_schemas();
        assert!(!schemas.is_empty());

        let names: Vec<&str> = schemas
            .iter()
            .filter_map(|s| s["function"]["name"].as_str())
            .collect();
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"grep_search"));
        assert!(names.contains(&"write_new_file"));
    }

    #[test]
    fn build_openai_messages_injects_tools() {
        let config = BridgeLlmConfig::default();
        let specs = default_text_tool_specs();
        let client = BridgeLlmClient::new(config, &specs).expect("build client");

        let messages = vec![
            AgentMessage::System("You are helpful.".to_string()),
            AgentMessage::User("Hello".to_string()),
        ];

        let openai = client.build_openai_messages(&messages);
        assert_eq!(openai.len(), 2);

        let sys_content = openai[0]["content"].as_str().unwrap();
        assert!(sys_content.contains("You are helpful."));
        assert!(sys_content.contains("Available Tools"));
        assert!(sys_content.contains("shell"));
    }

    #[test]
    fn build_openai_messages_formats_tool_results() {
        let config = BridgeLlmConfig::default();
        let client = BridgeLlmClient::new(config, &[]).expect("build client");

        let messages = vec![
            AgentMessage::System("sys".to_string()),
            AgentMessage::ToolResult {
                call_id: "c1".to_string(),
                output: "file contents here".to_string(),
            },
        ];

        let openai = client.build_openai_messages(&messages);
        // Tool results become user messages for text-based models.
        assert_eq!(openai[1]["role"], "user");
        let content = openai[1]["content"].as_str().unwrap();
        assert!(content.contains("Tool Result"));
        assert!(content.contains("file contents here"));
    }

    #[tokio::test]
    async fn exec_read_file_works() {
        let executor = BridgeToolExecutor::new(BridgeToolConfig::default());

        // Read this test file itself.
        let this_file = file!();
        // Construct the path relative to workspace.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let full_path = format!("{manifest_dir}/{this_file}");

        let result = executor
            .execute(&ToolCallRequest {
                call_id: "test".to_string(),
                name: "read_file".to_string(),
                arguments: serde_json::json!({"file_path": full_path, "limit": 5}),
            })
            .await
            .expect("execute");

        assert!(!result.is_error);
        assert!(result.output.contains("Bridge between"));
    }

    #[tokio::test]
    async fn exec_list_directory_works() {
        let executor = BridgeToolExecutor::new(BridgeToolConfig::default());

        let result = executor
            .execute(&ToolCallRequest {
                call_id: "test".to_string(),
                name: "list_directory".to_string(),
                arguments: serde_json::json!({"path": "."}),
            })
            .await
            .expect("execute");

        assert!(!result.is_error);
        // Should list some files/dirs.
        assert!(!result.output.is_empty());
    }

    #[tokio::test]
    async fn exec_unknown_tool_returns_error() {
        let executor = BridgeToolExecutor::new(BridgeToolConfig::default());

        let result = executor
            .execute(&ToolCallRequest {
                call_id: "test".to_string(),
                name: "nonexistent_tool".to_string(),
                arguments: serde_json::json!({}),
            })
            .await
            .expect("execute");

        assert!(result.is_error);
        assert!(result.output.contains("unknown tool"));
    }

    #[tokio::test]
    async fn exec_write_and_read_roundtrip() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let file_path = tmp.path().join("test_output.txt");
        let file_path_str = file_path.to_str().unwrap();

        let executor = BridgeToolExecutor::new(BridgeToolConfig::default());

        // Write.
        let write_result = executor
            .execute(&ToolCallRequest {
                call_id: "w1".to_string(),
                name: "write_new_file".to_string(),
                arguments: serde_json::json!({
                    "file_path": file_path_str,
                    "content": "hello from agent loop bridge test"
                }),
            })
            .await
            .expect("execute write");

        assert!(!write_result.is_error);
        assert!(write_result.output.contains("File written"));

        // Read back.
        let read_result = executor
            .execute(&ToolCallRequest {
                call_id: "r1".to_string(),
                name: "read_file".to_string(),
                arguments: serde_json::json!({"file_path": file_path_str}),
            })
            .await
            .expect("execute read");

        assert!(!read_result.is_error);
        assert!(read_result
            .output
            .contains("hello from agent loop bridge test"));
    }

    #[tokio::test]
    async fn full_agent_loop_with_bridge() {
        use super::super::{AgentLoop, AgentLoopConfig, StopReason};
        use std::sync::Mutex;

        // Mock LLM that returns a final text response (no tool calls).
        struct DirectLlm;

        #[async_trait::async_trait]
        impl AgentLlmClient for DirectLlm {
            async fn think(
                &self,
                _messages: &[AgentMessage],
                _tools: &[serde_json::Value],
            ) -> Result<ThinkResponse> {
                Ok(ThinkResponse {
                    content: "The answer is 42.".to_string(),
                    tool_calls: vec![],
                })
            }
        }

        let llm = Arc::new(DirectLlm);
        let tools = Arc::new(BridgeToolExecutor::new(BridgeToolConfig::default()));
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let cancel = tokio_util::sync::CancellationToken::new();
        let result = agent
            .run(
                "You are a test assistant.",
                "What is the answer?",
                move |ev| {
                    events_clone.lock().unwrap().push(format!("{ev:?}"));
                },
                cancel,
            )
            .await
            .expect("run");

        assert_eq!(result.stop_reason, StopReason::Complete);
        assert_eq!(result.response, "The answer is 42.");
        assert_eq!(result.iterations, 1);

        let events = events.lock().unwrap();
        assert!(events.len() >= 2); // Thinking + FinalResponse
    }

    #[tokio::test]
    async fn full_agent_loop_with_tool_call() {
        use super::super::{AgentLoop, AgentLoopConfig, StopReason};

        // Mock LLM: first call returns a tool call, second returns final answer.
        struct ToolCallingLlm {
            call_count: std::sync::atomic::AtomicUsize,
        }

        #[async_trait::async_trait]
        impl AgentLlmClient for ToolCallingLlm {
            async fn think(
                &self,
                _messages: &[AgentMessage],
                _tools: &[serde_json::Value],
            ) -> Result<ThinkResponse> {
                let count = self
                    .call_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count == 0 {
                    // First call: list the current directory.
                    Ok(ThinkResponse {
                        content: "Let me check the files.".to_string(),
                        tool_calls: vec![ToolCallRequest {
                            call_id: "call_1".to_string(),
                            name: "list_directory".to_string(),
                            arguments: serde_json::json!({"path": "."}),
                        }],
                    })
                } else {
                    // Second call: final answer.
                    Ok(ThinkResponse {
                        content: "I found the files.".to_string(),
                        tool_calls: vec![],
                    })
                }
            }
        }

        let llm = Arc::new(ToolCallingLlm {
            call_count: std::sync::atomic::AtomicUsize::new(0),
        });
        let tools = Arc::new(BridgeToolExecutor::new(BridgeToolConfig::default()));
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let cancel = tokio_util::sync::CancellationToken::new();
        let result = agent
            .run("system", "What files are here?", |_| {}, cancel)
            .await
            .expect("run");

        assert_eq!(result.stop_reason, StopReason::Complete);
        assert_eq!(result.response, "I found the files.");
        assert_eq!(result.iterations, 2);
        assert_eq!(result.tools_used, vec!["list_directory"]);
    }
}
