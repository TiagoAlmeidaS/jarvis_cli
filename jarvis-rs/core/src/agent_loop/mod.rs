//! Agentic Loop — Client-side Think → Execute → Observe → Repeat.
//!
//! This module implements a client-managed agentic loop that enables
//! autonomous multi-step tool use, independent of whether the underlying
//! LLM supports native function calling. The loop:
//!
//! 1. **THINK**: Sends the current context to the LLM.
//! 2. **EXECUTE**: If the LLM responds with tool calls, dispatches them.
//! 3. **OBSERVE**: Captures results and appends to context.
//! 4. **DECIDE**: If the LLM responds with plain text (no tool calls), stops.
//! 5. **REPEAT**: Otherwise goes back to step 1.
//!
//! Safety: iteration count, wall-clock timeout, cancellation token, and
//! context-window management prevent runaway loops.

pub mod bridge;
pub mod context;
pub mod events;

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio_util::sync::CancellationToken;

pub use context::ContextStrategy;
pub use events::AgentEvent;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Tunable knobs for the agentic loop.
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Maximum Think→Execute→Observe iterations before forced stop.
    pub max_iterations: usize,
    /// Maximum wall-clock time for the entire loop.
    pub max_duration: Duration,
    /// Maximum estimated context tokens before compaction kicks in.
    pub max_context_tokens: usize,
    /// Strategy for keeping context within the token budget.
    pub context_strategy: ContextStrategy,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            max_duration: Duration::from_secs(300), // 5 minutes
            max_context_tokens: 32_000,
            context_strategy: ContextStrategy::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// Outcome of running the agentic loop.
#[derive(Debug, Clone)]
pub struct AgentLoopResult {
    /// The final textual response from the LLM (may be empty on timeout/cancel).
    pub response: String,
    /// Total iterations completed.
    pub iterations: usize,
    /// Names of tools that were called (de-duplicated, in order).
    pub tools_used: Vec<String>,
    /// Why the loop terminated.
    pub stop_reason: StopReason,
    /// Wall-clock duration of the loop.
    pub elapsed: Duration,
}

/// Reason the loop terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// LLM produced a final response without requesting tools.
    Complete,
    /// Hit `max_iterations`.
    MaxIterations,
    /// Hit `max_duration`.
    Timeout,
    /// Cancelled via `CancellationToken`.
    Cancelled,
    /// Unrecoverable error during a tool call or LLM request.
    Error,
}

// ---------------------------------------------------------------------------
// Tool traits (abstracted so callers can plug different implementations)
// ---------------------------------------------------------------------------

/// A single tool-call request produced by the LLM.
#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    pub call_id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Result of executing a single tool call.
#[derive(Debug, Clone)]
pub struct ToolCallResult {
    pub call_id: String,
    pub name: String,
    pub output: String,
    pub is_error: bool,
}

/// LLM response from a single "think" step.
#[derive(Debug, Clone)]
pub struct ThinkResponse {
    /// Text content produced by the LLM (may be empty when only tool_calls present).
    pub content: String,
    /// Tool calls requested by the LLM (empty = final answer).
    pub tool_calls: Vec<ToolCallRequest>,
}

/// Abstraction over the LLM client so tests can inject a mock.
#[async_trait::async_trait]
pub trait AgentLlmClient: Send + Sync {
    /// Send the current message history and available tools to the LLM.
    async fn think(
        &self,
        messages: &[AgentMessage],
        tools: &[serde_json::Value],
    ) -> Result<ThinkResponse>;
}

/// Abstraction over the tool executor so tests can inject a mock.
#[async_trait::async_trait]
pub trait AgentToolExecutor: Send + Sync {
    /// Available tools as JSON schemas (sent to the LLM).
    fn tool_schemas(&self) -> Vec<serde_json::Value>;

    /// Execute a single tool call and return its output.
    async fn execute(&self, call: &ToolCallRequest) -> Result<ToolCallResult>;
}

// ---------------------------------------------------------------------------
// Message types (lightweight, not the full protocol types)
// ---------------------------------------------------------------------------

/// A message in the agentic loop conversation.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    System(String),
    User(String),
    Assistant(AssistantContent),
    ToolResult { call_id: String, output: String },
}

/// Content of an assistant message.
#[derive(Debug, Clone)]
pub struct AssistantContent {
    pub text: String,
    pub tool_calls: Vec<ToolCallRequest>,
}

// ---------------------------------------------------------------------------
// The loop
// ---------------------------------------------------------------------------

/// The agentic loop controller.
pub struct AgentLoop<L: AgentLlmClient, T: AgentToolExecutor> {
    llm: Arc<L>,
    tools: Arc<T>,
    config: AgentLoopConfig,
}

impl<L: AgentLlmClient, T: AgentToolExecutor> AgentLoop<L, T> {
    pub fn new(llm: Arc<L>, tools: Arc<T>, config: AgentLoopConfig) -> Self {
        Self { llm, tools, config }
    }

    /// Run the agentic loop.
    ///
    /// `on_event` is called synchronously on each lifecycle event so the
    /// caller (e.g. TUI) can render progress.
    pub async fn run<F>(
        &self,
        system_prompt: &str,
        user_message: &str,
        on_event: F,
        cancel: CancellationToken,
    ) -> Result<AgentLoopResult>
    where
        F: Fn(AgentEvent) + Send + Sync,
    {
        let mut messages: Vec<AgentMessage> = vec![
            AgentMessage::System(system_prompt.to_string()),
            AgentMessage::User(user_message.to_string()),
        ];

        let tool_schemas = self.tools.tool_schemas();
        let mut iteration: usize = 0;
        let mut tools_used: Vec<String> = Vec::new();
        let start = Instant::now();

        loop {
            // ---- Stop conditions ----
            if iteration >= self.config.max_iterations {
                on_event(AgentEvent::MaxIterationsReached {
                    iterations: iteration,
                });
                return Ok(AgentLoopResult {
                    response: String::new(),
                    iterations: iteration,
                    tools_used,
                    stop_reason: StopReason::MaxIterations,
                    elapsed: start.elapsed(),
                });
            }

            if start.elapsed() > self.config.max_duration {
                on_event(AgentEvent::Timeout {
                    elapsed: start.elapsed(),
                });
                return Ok(AgentLoopResult {
                    response: String::new(),
                    iterations: iteration,
                    tools_used,
                    stop_reason: StopReason::Timeout,
                    elapsed: start.elapsed(),
                });
            }

            if cancel.is_cancelled() {
                on_event(AgentEvent::Cancelled);
                return Ok(AgentLoopResult {
                    response: String::new(),
                    iterations: iteration,
                    tools_used,
                    stop_reason: StopReason::Cancelled,
                    elapsed: start.elapsed(),
                });
            }

            // ---- THINK ----
            on_event(AgentEvent::Thinking {
                iteration,
                max_iterations: self.config.max_iterations,
            });

            let response = match self.llm.think(&messages, &tool_schemas).await {
                Ok(r) => r,
                Err(e) => {
                    on_event(AgentEvent::Error {
                        message: format!("{e:#}"),
                    });
                    return Ok(AgentLoopResult {
                        response: String::new(),
                        iterations: iteration,
                        tools_used,
                        stop_reason: StopReason::Error,
                        elapsed: start.elapsed(),
                    });
                }
            };

            // ---- DECIDE: no tool calls = done ----
            if response.tool_calls.is_empty() {
                messages.push(AgentMessage::Assistant(AssistantContent {
                    text: response.content.clone(),
                    tool_calls: vec![],
                }));
                on_event(AgentEvent::FinalResponse {
                    content: response.content.clone(),
                    iteration,
                });
                return Ok(AgentLoopResult {
                    response: response.content,
                    iterations: iteration + 1,
                    tools_used,
                    stop_reason: StopReason::Complete,
                    elapsed: start.elapsed(),
                });
            }

            // ---- Record assistant message with tool calls ----
            messages.push(AgentMessage::Assistant(AssistantContent {
                text: response.content.clone(),
                tool_calls: response.tool_calls.clone(),
            }));

            // ---- EXECUTE each tool call ----
            for call in &response.tool_calls {
                on_event(AgentEvent::ExecutingTool {
                    name: call.name.clone(),
                    arguments: call.arguments.to_string(),
                    iteration,
                });

                let result = match self.tools.execute(call).await {
                    Ok(r) => r,
                    Err(e) => ToolCallResult {
                        call_id: call.call_id.clone(),
                        name: call.name.clone(),
                        output: format!("Error: {e:#}"),
                        is_error: true,
                    },
                };

                on_event(AgentEvent::ToolResult {
                    name: result.name.clone(),
                    output_preview: context::truncate_for_preview(&result.output, 200),
                    is_error: result.is_error,
                    iteration,
                });

                // Track unique tool names.
                if !tools_used.contains(&result.name) {
                    tools_used.push(result.name.clone());
                }

                // ---- OBSERVE: add result to context ----
                messages.push(AgentMessage::ToolResult {
                    call_id: result.call_id,
                    output: result.output,
                });
            }

            // ---- Context management ----
            let estimated_tokens = context::estimate_tokens(&messages);
            if estimated_tokens > self.config.max_context_tokens {
                on_event(AgentEvent::CompactingContext {
                    estimated_tokens,
                    max_tokens: self.config.max_context_tokens,
                });
                context::compact_messages(
                    &mut messages,
                    &self.config.context_strategy,
                    self.config.max_context_tokens,
                );
            }

            iteration += 1;
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
    use std::sync::Mutex;

    // Mock LLM client that returns predetermined responses.
    struct MockLlm {
        responses: Mutex<Vec<ThinkResponse>>,
    }

    impl MockLlm {
        fn new(responses: Vec<ThinkResponse>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    #[async_trait::async_trait]
    impl AgentLlmClient for MockLlm {
        async fn think(
            &self,
            _messages: &[AgentMessage],
            _tools: &[serde_json::Value],
        ) -> Result<ThinkResponse> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                Ok(ThinkResponse {
                    content: "(no more mock responses)".to_string(),
                    tool_calls: vec![],
                })
            } else {
                Ok(responses.remove(0))
            }
        }
    }

    // Mock tool executor.
    struct MockTools;

    #[async_trait::async_trait]
    impl AgentToolExecutor for MockTools {
        fn tool_schemas(&self) -> Vec<serde_json::Value> {
            vec![serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_directory",
                    "description": "List files in a directory",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"}
                        },
                        "required": ["path"]
                    }
                }
            })]
        }

        async fn execute(&self, call: &ToolCallRequest) -> Result<ToolCallResult> {
            Ok(ToolCallResult {
                call_id: call.call_id.clone(),
                name: call.name.clone(),
                output: format!(
                    "Mock output for {} with args: {}",
                    call.name, call.arguments
                ),
                is_error: false,
            })
        }
    }

    fn make_cancel() -> CancellationToken {
        CancellationToken::new()
    }

    #[tokio::test]
    async fn test_loop_no_tools_needed() {
        let llm = Arc::new(MockLlm::new(vec![ThinkResponse {
            content: "The answer is 42.".to_string(),
            tool_calls: vec![],
        }]));
        let tools = Arc::new(MockTools);
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let result = agent
            .run("system", "What is the answer?", |_| {}, make_cancel())
            .await
            .unwrap();

        assert_eq!(result.response, "The answer is 42.");
        assert_eq!(result.iterations, 1);
        assert!(result.tools_used.is_empty());
        assert_eq!(result.stop_reason, StopReason::Complete);
    }

    #[tokio::test]
    async fn test_loop_single_tool_call() {
        let llm = Arc::new(MockLlm::new(vec![
            // Iteration 0: call a tool.
            ThinkResponse {
                content: "Let me check.".to_string(),
                tool_calls: vec![ToolCallRequest {
                    call_id: "call_1".to_string(),
                    name: "list_directory".to_string(),
                    arguments: serde_json::json!({"path": "."}),
                }],
            },
            // Iteration 1: final answer.
            ThinkResponse {
                content: "The directory has 3 files.".to_string(),
                tool_calls: vec![],
            },
        ]));
        let tools = Arc::new(MockTools);
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let result = agent
            .run("system", "List files", |_| {}, make_cancel())
            .await
            .unwrap();

        assert_eq!(result.response, "The directory has 3 files.");
        assert_eq!(result.iterations, 2);
        assert_eq!(result.tools_used, vec!["list_directory"]);
        assert_eq!(result.stop_reason, StopReason::Complete);
    }

    #[tokio::test]
    async fn test_loop_multi_tool_calls() {
        let llm = Arc::new(MockLlm::new(vec![
            ThinkResponse {
                content: "".to_string(),
                tool_calls: vec![
                    ToolCallRequest {
                        call_id: "c1".to_string(),
                        name: "list_directory".to_string(),
                        arguments: serde_json::json!({"path": "."}),
                    },
                    ToolCallRequest {
                        call_id: "c2".to_string(),
                        name: "list_directory".to_string(),
                        arguments: serde_json::json!({"path": "src"}),
                    },
                ],
            },
            ThinkResponse {
                content: "Found files in both dirs.".to_string(),
                tool_calls: vec![],
            },
        ]));
        let tools = Arc::new(MockTools);
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let result = agent
            .run("system", "Check both", |_| {}, make_cancel())
            .await
            .unwrap();

        assert_eq!(result.iterations, 2);
        // list_directory appears once in tools_used (deduplicated).
        assert_eq!(result.tools_used, vec!["list_directory"]);
        assert_eq!(result.stop_reason, StopReason::Complete);
    }

    #[tokio::test]
    async fn test_loop_max_iterations() {
        // LLM always requests tools, never gives a final answer.
        let mut responses = Vec::new();
        for i in 0..30 {
            responses.push(ThinkResponse {
                content: "".to_string(),
                tool_calls: vec![ToolCallRequest {
                    call_id: format!("call_{i}"),
                    name: "list_directory".to_string(),
                    arguments: serde_json::json!({"path": "."}),
                }],
            });
        }

        let llm = Arc::new(MockLlm::new(responses));
        let tools = Arc::new(MockTools);
        let config = AgentLoopConfig {
            max_iterations: 3,
            ..Default::default()
        };
        let agent = AgentLoop::new(llm, tools, config);

        let result = agent
            .run("system", "Go!", |_| {}, make_cancel())
            .await
            .unwrap();

        assert_eq!(result.iterations, 3);
        assert_eq!(result.stop_reason, StopReason::MaxIterations);
        assert!(result.response.is_empty());
    }

    #[tokio::test]
    async fn test_loop_cancelled() {
        let llm = Arc::new(MockLlm::new(vec![]));
        let tools = Arc::new(MockTools);
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let cancel = CancellationToken::new();
        cancel.cancel(); // Pre-cancel.

        let result = agent.run("system", "Hello", |_| {}, cancel).await.unwrap();

        assert_eq!(result.stop_reason, StopReason::Cancelled);
        assert_eq!(result.iterations, 0);
    }

    #[tokio::test]
    async fn test_events_emitted() {
        let llm = Arc::new(MockLlm::new(vec![
            ThinkResponse {
                content: "".to_string(),
                tool_calls: vec![ToolCallRequest {
                    call_id: "c1".to_string(),
                    name: "list_directory".to_string(),
                    arguments: serde_json::json!({"path": "."}),
                }],
            },
            ThinkResponse {
                content: "Done.".to_string(),
                tool_calls: vec![],
            },
        ]));
        let tools = Arc::new(MockTools);
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let result = agent
            .run(
                "system",
                "Test",
                move |ev| {
                    events_clone.lock().unwrap().push(format!("{ev:?}"));
                },
                make_cancel(),
            )
            .await
            .unwrap();

        assert_eq!(result.stop_reason, StopReason::Complete);

        let events = events.lock().unwrap();
        // Should have: Thinking(0), ExecutingTool, ToolResult, Thinking(1), FinalResponse.
        assert!(events.len() >= 4);
        assert!(events[0].contains("Thinking"));
        assert!(events[1].contains("ExecutingTool"));
        assert!(events[2].contains("ToolResult"));
        assert!(events[3].contains("Thinking"));
    }

    #[tokio::test]
    async fn test_loop_error_from_llm() {
        struct FailingLlm;

        #[async_trait::async_trait]
        impl AgentLlmClient for FailingLlm {
            async fn think(
                &self,
                _messages: &[AgentMessage],
                _tools: &[serde_json::Value],
            ) -> Result<ThinkResponse> {
                anyhow::bail!("LLM unavailable")
            }
        }

        let llm = Arc::new(FailingLlm);
        let tools = Arc::new(MockTools);
        let agent = AgentLoop::new(llm, tools, AgentLoopConfig::default());

        let result = agent
            .run("system", "Hello", |_| {}, make_cancel())
            .await
            .unwrap();

        assert_eq!(result.stop_reason, StopReason::Error);
    }
}
