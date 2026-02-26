//! Events emitted by the agentic loop for UI / telemetry consumption.

use std::time::Duration;

/// Lifecycle event emitted by [`super::AgentLoop::run`].
///
/// The TUI (or any other consumer) receives these via the `on_event`
/// callback to render real-time progress.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent is calling the LLM (start of a "think" step).
    Thinking {
        iteration: usize,
        max_iterations: usize,
    },

    /// Agent is executing a tool.
    ExecutingTool {
        name: String,
        arguments: String,
        iteration: usize,
    },

    /// Tool execution completed.
    ToolResult {
        name: String,
        output_preview: String,
        is_error: bool,
        iteration: usize,
    },

    /// Context window exceeded budget; compacting.
    CompactingContext {
        estimated_tokens: usize,
        max_tokens: usize,
    },

    /// Agent produced a final textual response.
    FinalResponse { content: String, iteration: usize },

    /// Loop stopped: max iterations reached.
    MaxIterationsReached { iterations: usize },

    /// Loop stopped: wall-clock timeout.
    Timeout { elapsed: Duration },

    /// Loop stopped: user cancelled.
    Cancelled,

    /// Unrecoverable error.
    Error { message: String },

    /// Intent detected from user message.
    IntentDetected {
        intent_type: String,
        confidence: f32,
    },

    /// Safety classifier issued a warning (medium risk) but allowed execution.
    SafetyWarning {
        tool_name: String,
        risk_level: String,
        reasoning: String,
    },

    /// Safety classifier blocked a tool call (high/critical risk).
    SafetyBlocked {
        tool_name: String,
        risk_level: String,
        reasoning: String,
    },

    /// Knowledge was extracted and stored from the current interaction.
    KnowledgeLearned {
        /// Number of new knowledge items stored.
        items_count: usize,
        /// Short summary of what was learned.
        summary: String,
    },

    /// Relevant knowledge was retrieved and injected into context.
    KnowledgeRetrieved {
        /// Number of knowledge items found.
        items_count: usize,
        /// The query used to search.
        query: String,
    },
}
