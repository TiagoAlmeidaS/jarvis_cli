//! Text-based tool calling for models without native function calling support.
//!
//! Some LLMs (e.g. smaller Mistral, LLaMA variants) don't support the OpenAI
//! `tools` / `tool_calls` protocol natively. This module provides:
//!
//! 1. **Prompt injection**: Generates tool descriptions that can be appended to
//!    the system prompt so the model knows which tools are available and how to
//!    invoke them.
//!
//! 2. **Text parser**: Extracts tool calls from the model's plain-text response
//!    (delimited by ` ```tool_call ``` ` fences) and converts them into the same
//!    `ToolCallRequest` structures the rest of the pipeline understands.
//!
//! 3. **Mode detection**: Heuristically decides whether a model/provider
//!    combination should use native or text-based calling.

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Tool calling mode
// ---------------------------------------------------------------------------

/// How tools should be presented to and parsed from a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallingMode {
    /// Model supports native function calling (OpenAI `tools` + `tool_calls`).
    Native,
    /// Model uses text-based tool calling (prompt injection + text parsing).
    TextBased,
    /// No tool calling — model only generates text.
    Disabled,
}

impl ToolCallingMode {
    /// Heuristically detect the tool calling mode for a model/provider pair.
    ///
    /// This is intentionally conservative: models that are *known* to lack
    /// function-calling support get `TextBased`; everything else defaults to
    /// `Native` because most OpenRouter / Ollama proxies translate the `tools`
    /// field for the underlying model.
    pub fn detect(model: &str, _provider: &str) -> Self {
        let m = model.to_lowercase();

        // Models known to NOT support native function calling reliably.
        // These benefit from text-based tool calling.
        if is_text_based_model(&m) {
            return Self::TextBased;
        }

        // Default: use native — the proxy/provider should handle translation.
        Self::Native
    }
}

/// Check if a model slug is known to lack native function calling.
fn is_text_based_model(model_lower: &str) -> bool {
    // Very small / instruction-tuned models without tool support.
    let text_only_patterns = [
        "mistral-nemo",
        "phi-3",
        "phi-2",
        "tinyllama",
        "stablelm",
        "gemma-2b",
        "gemma-7b",
        "yi-6b",
        "yi-9b",
    ];

    text_only_patterns
        .iter()
        .any(|pat| model_lower.contains(pat))
}

// ---------------------------------------------------------------------------
// Prompt injection
// ---------------------------------------------------------------------------

/// A lightweight tool description used for prompt injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Vec<TextToolParam>,
}

/// A single parameter of a text tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToolParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Build the text to inject into the system prompt that describes available tools.
pub fn build_tool_prompt_injection(tools: &[TextToolSpec]) -> String {
    if tools.is_empty() {
        return String::new();
    }

    let mut out = String::from(
        "\n\n## Available Tools\n\n\
         You have access to the following tools. You MUST use these tools to \
         accomplish tasks. NEVER ask the user for file paths, directory locations, \
         or information you can discover yourself using these tools.\n\n\
         **IMPORTANT RULES:**\n\
         - When the user asks you to analyze, read, search, or explore something, \
           USE the tools immediately. Do NOT ask for clarification.\n\
         - Use `list_directory` to explore project structure.\n\
         - Use `read_file` to read file contents.\n\
         - Use `grep_search` to search for patterns in files.\n\
         - Use `shell` to run commands.\n\
         - Always act autonomously. You are an agent — take action, don't ask.\n\n\
         To call a tool, include a JSON block inside a ```tool_call fence:\n\n\
         ```tool_call\n\
         {\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n\
         ```\n\n\
         You may call multiple tools in a single response by including multiple \
         ```tool_call blocks. After receiving tool results, continue your analysis \
         or call more tools as needed.\n\n\
         ### Tools\n\n",
    );

    for tool in tools {
        out.push_str(&format!("**{}** — {}\n", tool.name, tool.description));
        if !tool.parameters.is_empty() {
            out.push_str("  Parameters:\n");
            for param in &tool.parameters {
                let req = if param.required {
                    "required"
                } else {
                    "optional"
                };
                out.push_str(&format!(
                    "  - `{}` ({}, {}): {}\n",
                    param.name, param.param_type, req, param.description
                ));
            }
        }
        out.push('\n');
    }

    out
}

/// Build default text tool specs for the core built-in tools.
pub fn default_text_tool_specs() -> Vec<TextToolSpec> {
    vec![
        TextToolSpec {
            name: "shell".to_string(),
            description: "Execute a shell command and return its output.".to_string(),
            parameters: vec![
                TextToolParam {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "The shell command to execute.".to_string(),
                    required: true,
                },
                TextToolParam {
                    name: "workdir".to_string(),
                    param_type: "string".to_string(),
                    description: "Working directory (optional, defaults to cwd).".to_string(),
                    required: false,
                },
            ],
        },
        TextToolSpec {
            name: "read_file".to_string(),
            description: "Read the contents of a file.".to_string(),
            parameters: vec![
                TextToolParam {
                    name: "file_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Absolute path to the file to read.".to_string(),
                    required: true,
                },
                TextToolParam {
                    name: "offset".to_string(),
                    param_type: "integer".to_string(),
                    description: "1-indexed line number to start from (default: 1).".to_string(),
                    required: false,
                },
                TextToolParam {
                    name: "limit".to_string(),
                    param_type: "integer".to_string(),
                    description: "Maximum number of lines to return (default: 2000).".to_string(),
                    required: false,
                },
            ],
        },
        TextToolSpec {
            name: "list_directory".to_string(),
            description: "List files and directories in a given path.".to_string(),
            parameters: vec![TextToolParam {
                name: "path".to_string(),
                param_type: "string".to_string(),
                description: "Absolute path to the directory to list.".to_string(),
                required: true,
            }],
        },
        TextToolSpec {
            name: "grep_search".to_string(),
            description: "Search for a pattern in files using ripgrep.".to_string(),
            parameters: vec![
                TextToolParam {
                    name: "pattern".to_string(),
                    param_type: "string".to_string(),
                    description: "Regex pattern to search for.".to_string(),
                    required: true,
                },
                TextToolParam {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Directory or file to search in.".to_string(),
                    required: false,
                },
                TextToolParam {
                    name: "include".to_string(),
                    param_type: "string".to_string(),
                    description: "Glob pattern for files to include (e.g. '*.rs').".to_string(),
                    required: false,
                },
            ],
        },
        TextToolSpec {
            name: "write_new_file".to_string(),
            description: "Create a new file with the given content.".to_string(),
            parameters: vec![
                TextToolParam {
                    name: "file_path".to_string(),
                    param_type: "string".to_string(),
                    description: "Absolute path for the new file.".to_string(),
                    required: true,
                },
                TextToolParam {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    description: "Content to write to the file.".to_string(),
                    required: true,
                },
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// Text parsing
// ---------------------------------------------------------------------------

/// A tool call extracted from model text output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedToolCall {
    /// Generated call ID (since text models don't produce one).
    pub call_id: String,
    /// Tool name.
    pub name: String,
    /// Arguments as a JSON value.
    pub arguments: serde_json::Value,
}

/// Parse tool calls from model text output.
///
/// Looks for blocks delimited by ` ```tool_call ` and ` ``` ` fences.
/// Each block should contain a JSON object with `name` and `arguments` fields.
///
/// Returns `(remaining_text, extracted_tool_calls)` where `remaining_text`
/// is the model's response with the tool_call blocks removed.
pub fn parse_tool_calls_from_text(text: &str) -> (String, Vec<ParsedToolCall>) {
    let mut tool_calls = Vec::new();
    let mut remaining = String::new();
    let mut chars = text.chars().peekable();
    let mut in_tool_block = false;
    let mut block_buf = String::new();
    let mut line_buf = String::new();

    // Simple line-based parser.
    for ch in text.chars() {
        if ch == '\n' {
            let trimmed = line_buf.trim();

            if !in_tool_block && is_tool_call_fence_start(trimmed) {
                in_tool_block = true;
                block_buf.clear();
                line_buf.clear();
                continue;
            }

            if in_tool_block && is_fence_end(trimmed) {
                // Try to parse the block as JSON.
                if let Some(tc) = try_parse_tool_call_json(&block_buf) {
                    tool_calls.push(tc);
                } else {
                    // Failed to parse — keep the text as-is.
                    remaining.push_str("```tool_call\n");
                    remaining.push_str(&block_buf);
                    remaining.push_str("```\n");
                }
                in_tool_block = false;
                block_buf.clear();
                line_buf.clear();
                continue;
            }

            if in_tool_block {
                block_buf.push_str(&line_buf);
                block_buf.push('\n');
            } else {
                remaining.push_str(&line_buf);
                remaining.push('\n');
            }

            line_buf.clear();
        } else {
            line_buf.push(ch);
        }
    }

    // Handle trailing content (no final newline).
    if !line_buf.is_empty() {
        if in_tool_block {
            block_buf.push_str(&line_buf);
            // Unterminated block — try parsing anyway.
            if let Some(tc) = try_parse_tool_call_json(&block_buf) {
                tool_calls.push(tc);
            } else {
                remaining.push_str("```tool_call\n");
                remaining.push_str(&block_buf);
            }
        } else {
            remaining.push_str(&line_buf);
        }
    }

    // Also try to detect inline JSON tool calls (some models don't use fences).
    if tool_calls.is_empty() {
        let inline = try_extract_inline_tool_calls(text);
        if !inline.is_empty() {
            tool_calls = inline;
            // For inline calls, keep the full text as the remaining text
            // since we can't cleanly excise inline JSON.
        }
    }

    (remaining.trim().to_string(), tool_calls)
}

/// Check if a trimmed line starts a tool_call fence.
fn is_tool_call_fence_start(line: &str) -> bool {
    let l = line.trim_start_matches('`');
    // Accept ```tool_call, ```tool, or just ``` with tool_call on the same line
    line.starts_with("```tool_call")
        || line.starts_with("```tool")
        || (line.starts_with("```") && line.contains("tool"))
}

/// Check if a trimmed line ends a fence.
fn is_fence_end(line: &str) -> bool {
    line == "```" || line == "````"
}

/// Try to parse a JSON block as a tool call.
fn try_parse_tool_call_json(json_str: &str) -> Option<ParsedToolCall> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try the standard format: {"name": "...", "arguments": {...}}
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let name = val.get("name")?.as_str()?;
        let arguments = val
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        return Some(ParsedToolCall {
            call_id: format!("text_{}", Uuid::new_v4()),
            name: name.to_string(),
            arguments,
        });
    }

    // Try alternative format: {"tool": "...", "input": {...}}
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let name = val
            .get("tool")
            .or_else(|| val.get("function"))
            .and_then(|v| v.as_str())?;
        let arguments = val
            .get("input")
            .or_else(|| val.get("params"))
            .or_else(|| val.get("args"))
            .cloned()
            .unwrap_or(serde_json::json!({}));
        return Some(ParsedToolCall {
            call_id: format!("text_{}", Uuid::new_v4()),
            name: name.to_string(),
            arguments,
        });
    }

    None
}

/// Try to extract tool calls from inline JSON in the text.
///
/// Some models may output tool calls without fences, like:
/// "I'll check the directory. {"name": "list_directory", "arguments": {"path": "."}}"
fn try_extract_inline_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    let mut calls = Vec::new();

    // Find JSON objects in the text by looking for balanced braces.
    let mut depth = 0i32;
    let mut start = None;

    for (i, ch) in text.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        let candidate = &text[s..=i];
                        if let Some(tc) = try_parse_tool_call_json(candidate) {
                            calls.push(tc);
                        }
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }

    calls
}

/// Format tool results for injection back into the conversation.
///
/// Used when sending tool outputs back to a text-based model that doesn't
/// understand the `tool` role.
pub fn format_tool_result_for_text(tool_name: &str, call_id: &str, output: &str) -> String {
    format!(
        "[Tool Result: {tool_name} (id: {call_id})]\n\
         {output}\n\
         [End Tool Result]"
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn detect_native_for_gpt4() {
        assert_eq!(
            ToolCallingMode::detect("gpt-4o", "openai"),
            ToolCallingMode::Native
        );
    }

    #[test]
    fn detect_text_based_for_mistral_nemo() {
        assert_eq!(
            ToolCallingMode::detect("mistralai/mistral-nemo", "openrouter"),
            ToolCallingMode::TextBased
        );
    }

    #[test]
    fn detect_native_for_unknown() {
        assert_eq!(
            ToolCallingMode::detect("some-new-model", "provider"),
            ToolCallingMode::Native
        );
    }

    #[test]
    fn parse_fenced_tool_call() {
        let text = "Let me check the directory.\n\n\
                     ```tool_call\n\
                     {\"name\": \"list_directory\", \"arguments\": {\"path\": \".\"}}\n\
                     ```\n\n\
                     I'll analyze the results.";

        let (remaining, calls) = parse_tool_calls_from_text(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "list_directory");
        assert_eq!(calls[0].arguments["path"], ".");
        assert!(remaining.contains("Let me check the directory."));
        assert!(remaining.contains("I'll analyze the results."));
        assert!(!remaining.contains("tool_call"));
    }

    #[test]
    fn parse_multiple_fenced_tool_calls() {
        let text = "First I'll list the dir.\n\n\
                     ```tool_call\n\
                     {\"name\": \"list_directory\", \"arguments\": {\"path\": \".\"}}\n\
                     ```\n\n\
                     Then read the file.\n\n\
                     ```tool_call\n\
                     {\"name\": \"read_file\", \"arguments\": {\"file_path\": \"src/main.rs\"}}\n\
                     ```\n";

        let (remaining, calls) = parse_tool_calls_from_text(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "list_directory");
        assert_eq!(calls[1].name, "read_file");
        assert!(remaining.contains("First I'll list the dir."));
        assert!(remaining.contains("Then read the file."));
    }

    #[test]
    fn parse_no_tool_calls() {
        let text = "This is just a regular response with no tool calls.";
        let (remaining, calls) = parse_tool_calls_from_text(text);
        assert!(calls.is_empty());
        assert_eq!(remaining, text);
    }

    #[test]
    fn parse_malformed_json_keeps_text() {
        let text = "```tool_call\n\
                     {this is not valid json}\n\
                     ```\n";
        let (remaining, calls) = parse_tool_calls_from_text(text);
        assert!(calls.is_empty());
        assert!(remaining.contains("tool_call"));
    }

    #[test]
    fn prompt_injection_generates_valid_text() {
        let tools = default_text_tool_specs();
        let injection = build_tool_prompt_injection(&tools);
        assert!(injection.contains("## Available Tools"));
        assert!(injection.contains("shell"));
        assert!(injection.contains("read_file"));
        assert!(injection.contains("list_directory"));
        assert!(injection.contains("grep_search"));
        assert!(injection.contains("```tool_call"));
    }

    #[test]
    fn format_tool_result() {
        let result = format_tool_result_for_text("shell", "call_123", "hello world");
        assert!(result.contains("[Tool Result: shell"));
        assert!(result.contains("hello world"));
        assert!(result.contains("[End Tool Result]"));
    }

    #[test]
    fn parse_inline_json_tool_call() {
        let text = "I'll check the files. {\"name\": \"list_directory\", \"arguments\": {\"path\": \"/tmp\"}} Let me see.";
        let (_, calls) = parse_tool_calls_from_text(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "list_directory");
    }
}
