//! Context management for the agentic loop.
//!
//! As the loop iterates, the message history grows. This module provides
//! strategies to keep the context within the LLM's token budget.

use super::AgentMessage;

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

/// How the loop should manage context when it approaches the token limit.
#[derive(Debug, Clone)]
pub enum ContextStrategy {
    /// Truncate individual tool results that exceed a character limit.
    TruncateToolResults {
        /// Maximum characters per tool result output.
        max_chars_per_result: usize,
    },
    /// Drop the oldest non-system messages, keeping only the last N.
    SlidingWindow {
        /// Minimum number of recent messages to keep.
        keep_last_n: usize,
    },
    /// Combination: first truncate long tool results, then apply sliding window.
    Hybrid {
        max_chars_per_result: usize,
        keep_last_n: usize,
    },
}

impl Default for ContextStrategy {
    fn default() -> Self {
        Self::Hybrid {
            max_chars_per_result: 4_000,
            keep_last_n: 20,
        }
    }
}

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Rough token estimate: ~4 characters per token (English average).
const CHARS_PER_TOKEN: usize = 4;

/// Estimate the total token count for a message list.
pub fn estimate_tokens(messages: &[AgentMessage]) -> usize {
    let total_chars: usize = messages
        .iter()
        .map(|m| match m {
            AgentMessage::System(s) | AgentMessage::User(s) => s.len(),
            AgentMessage::Assistant(a) => {
                a.text.len()
                    + a.tool_calls
                        .iter()
                        .map(|tc| tc.name.len() + tc.arguments.to_string().len())
                        .sum::<usize>()
            }
            AgentMessage::ToolResult { output, .. } => output.len(),
        })
        .sum();

    total_chars / CHARS_PER_TOKEN
}

// ---------------------------------------------------------------------------
// Compaction
// ---------------------------------------------------------------------------

/// Apply the configured strategy to reduce message context size.
pub fn compact_messages(
    messages: &mut Vec<AgentMessage>,
    strategy: &ContextStrategy,
    _max_tokens: usize,
) {
    match strategy {
        ContextStrategy::TruncateToolResults {
            max_chars_per_result,
        } => {
            truncate_tool_results(messages, *max_chars_per_result);
        }
        ContextStrategy::SlidingWindow { keep_last_n } => {
            apply_sliding_window(messages, *keep_last_n);
        }
        ContextStrategy::Hybrid {
            max_chars_per_result,
            keep_last_n,
        } => {
            truncate_tool_results(messages, *max_chars_per_result);
            apply_sliding_window(messages, *keep_last_n);
        }
    }
}

/// Truncate tool result outputs that exceed `max_chars`.
fn truncate_tool_results(messages: &mut [AgentMessage], max_chars: usize) {
    for msg in messages.iter_mut() {
        if let AgentMessage::ToolResult { output, .. } = msg {
            if output.len() > max_chars {
                *output = truncate_text(output, max_chars);
            }
        }
    }
}

/// Keep only the system message + the last N non-system messages.
fn apply_sliding_window(messages: &mut Vec<AgentMessage>, keep_last_n: usize) {
    if messages.len() <= keep_last_n + 1 {
        return;
    }

    // Separate system messages (index 0 typically) from the rest.
    let system_count = messages
        .iter()
        .take_while(|m| matches!(m, AgentMessage::System(_)))
        .count();

    let non_system_count = messages.len() - system_count;
    if non_system_count <= keep_last_n {
        return;
    }

    let drop_count = non_system_count - keep_last_n;
    // Remove `drop_count` messages starting right after the system messages.
    messages.drain(system_count..system_count + drop_count);
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

/// Truncate text showing head + tail with a notice in the middle.
fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let half = max_chars / 2;
    let omitted = text.len() - max_chars;
    format!(
        "{}\n\n... ({omitted} characters truncated) ...\n\n{}",
        &text[..half],
        &text[text.len() - half..],
    )
}

/// Truncate for a short preview (e.g. event logging).
pub fn truncate_for_preview(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}...", &text[..max_chars])
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_loop::AssistantContent;
    use crate::agent_loop::ToolCallRequest;
    use pretty_assertions::assert_eq;

    #[test]
    fn estimate_tokens_simple() {
        let messages = vec![
            AgentMessage::System("You are helpful.".to_string()),
            AgentMessage::User("Hello world".to_string()),
        ];
        // "You are helpful." = 16 chars, "Hello world" = 11 chars = 27 chars total.
        // 27 / 4 = 6 tokens (integer division).
        assert_eq!(estimate_tokens(&messages), 6);
    }

    #[test]
    fn truncate_text_short_passthrough() {
        let text = "short text";
        assert_eq!(truncate_text(text, 100), "short text");
    }

    #[test]
    fn truncate_text_long() {
        let text = "a".repeat(1000);
        let result = truncate_text(&text, 200);
        assert!(result.len() < 1000);
        assert!(result.contains("truncated"));
    }

    #[test]
    fn truncate_for_preview_short() {
        assert_eq!(truncate_for_preview("hello", 10), "hello");
    }

    #[test]
    fn truncate_for_preview_long() {
        let result = truncate_for_preview("a long string here", 6);
        assert_eq!(result, "a long...");
    }

    #[test]
    fn sliding_window_keeps_system_and_recent() {
        let mut messages = vec![
            AgentMessage::System("sys".to_string()),
            AgentMessage::User("u1".to_string()),
            AgentMessage::Assistant(AssistantContent {
                text: "a1".to_string(),
                tool_calls: vec![],
            }),
            AgentMessage::User("u2".to_string()),
            AgentMessage::Assistant(AssistantContent {
                text: "a2".to_string(),
                tool_calls: vec![],
            }),
            AgentMessage::User("u3".to_string()),
        ];

        apply_sliding_window(&mut messages, 3);

        // Should keep: System + last 3 non-system messages.
        assert_eq!(messages.len(), 4); // 1 system + 3 kept
        assert!(matches!(&messages[0], AgentMessage::System(_)));
    }

    #[test]
    fn truncate_tool_results_respects_limit() {
        let long_output = "x".repeat(10_000);
        let mut messages = vec![AgentMessage::ToolResult {
            call_id: "c1".to_string(),
            output: long_output,
        }];

        truncate_tool_results(&mut messages, 500);

        if let AgentMessage::ToolResult { output, .. } = &messages[0] {
            assert!(output.len() < 10_000);
            assert!(output.contains("truncated"));
        } else {
            panic!("expected ToolResult");
        }
    }

    #[test]
    fn compact_hybrid() {
        let long_output = "y".repeat(8_000);
        let mut messages = vec![
            AgentMessage::System("sys".to_string()),
            AgentMessage::User("u1".to_string()),
            AgentMessage::ToolResult {
                call_id: "c1".to_string(),
                output: long_output,
            },
            AgentMessage::User("u2".to_string()),
            AgentMessage::User("u3".to_string()),
            AgentMessage::User("u4".to_string()),
        ];

        compact_messages(
            &mut messages,
            &ContextStrategy::Hybrid {
                max_chars_per_result: 500,
                keep_last_n: 3,
            },
            10_000,
        );

        // Tool result should be truncated.
        // Also, sliding window should keep system + last 3.
        assert_eq!(messages.len(), 4); // system + 3
    }
}
