use crate::common::Reasoning;
use crate::common::ResponsesApiRequest;
use crate::common::TextControls;
use crate::error::ApiError;
use crate::provider::Provider;
use crate::requests::headers::build_conversation_headers;
use crate::requests::headers::insert_header;
use crate::requests::headers::subagent_header;
use jarvis_protocol::models::ResponseItem;
use jarvis_protocol::protocol::SessionSource;
use http::HeaderMap;
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Compression {
    #[default]
    None,
    Zstd,
}

/// Assembled request body plus headers for a Responses stream request.
pub struct ResponsesRequest {
    pub body: Value,
    pub headers: HeaderMap,
    pub compression: Compression,
    pub path: String,
}

#[derive(Default)]
pub struct ResponsesRequestBuilder<'a> {
    model: Option<&'a str>,
    instructions: Option<&'a str>,
    input: Option<&'a [ResponseItem]>,
    tools: Option<&'a [Value]>,
    parallel_tool_calls: bool,
    reasoning: Option<Reasoning>,
    include: Vec<String>,
    prompt_cache_key: Option<String>,
    text: Option<TextControls>,
    conversation_id: Option<String>,
    session_source: Option<SessionSource>,
    store_override: Option<bool>,
    headers: HeaderMap,
    compression: Compression,
}

impl<'a> ResponsesRequestBuilder<'a> {
    pub fn new(model: &'a str, instructions: &'a str, input: &'a [ResponseItem]) -> Self {
        Self {
            model: Some(model),
            instructions: Some(instructions),
            input: Some(input),
            ..Default::default()
        }
    }

    pub fn tools(mut self, tools: &'a [Value]) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = enabled;
        self
    }

    pub fn reasoning(mut self, reasoning: Option<Reasoning>) -> Self {
        self.reasoning = reasoning;
        self
    }

    pub fn include(mut self, include: Vec<String>) -> Self {
        self.include = include;
        self
    }

    pub fn prompt_cache_key(mut self, key: Option<String>) -> Self {
        self.prompt_cache_key = key;
        self
    }

    pub fn text(mut self, text: Option<TextControls>) -> Self {
        self.text = text;
        self
    }

    pub fn conversation(mut self, conversation_id: Option<String>) -> Self {
        self.conversation_id = conversation_id;
        self
    }

    pub fn session_source(mut self, source: Option<SessionSource>) -> Self {
        self.session_source = source;
        self
    }

    pub fn store_override(mut self, store: Option<bool>) -> Self {
        self.store_override = store;
        self
    }

    pub fn extra_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    pub fn compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    pub fn build(self, provider: &Provider) -> Result<ResponsesRequest, ApiError> {
        let model = self
            .model
            .ok_or_else(|| ApiError::Stream("missing model for responses request".into()))?;
        let instructions = self
            .instructions
            .ok_or_else(|| ApiError::Stream("missing instructions for responses request".into()))?;
        let input = self
            .input
            .ok_or_else(|| ApiError::Stream("missing input for responses request".into()))?;
        let tools = self.tools.unwrap_or_default();

        let store = self
            .store_override
            .unwrap_or_else(|| provider.is_azure_responses_endpoint());

        let req = ResponsesApiRequest {
            model,
            instructions,
            input,
            tools,
            tool_choice: "auto",
            parallel_tool_calls: self.parallel_tool_calls,
            reasoning: self.reasoning,
            store,
            stream: true,
            include: self.include,
            prompt_cache_key: self.prompt_cache_key,
            text: self.text,
        };

        let mut body = serde_json::to_value(&req)
            .map_err(|e| ApiError::Stream(format!("failed to encode responses request: {e}")))?;

        if store && provider.is_azure_responses_endpoint() {
            attach_item_ids(&mut body, input);
        }

        // Convert to Chat Completions format for Databricks
        if provider.name.to_lowercase() == "databricks" {
            body = convert_to_chat_format(&body, model, instructions, input)?;
        }

        let mut headers = self.headers;
        headers.extend(build_conversation_headers(self.conversation_id));
        if let Some(subagent) = subagent_header(&self.session_source) {
            insert_header(&mut headers, "x-openai-subagent", &subagent);
        }

        // Build path dynamically for Databricks provider
        let path = if provider.name.to_lowercase() == "databricks" {
            // For Databricks, construct path: serving-endpoints/{model}/invocations
            format!("serving-endpoints/{}/invocations", model)
        } else {
            // For other providers (OpenAI, etc.), use standard "responses" path
            "responses".to_string()
        };

        Ok(ResponsesRequest {
            body,
            headers,
            compression: self.compression,
            path,
        })
    }
}

/// Converts Responses API format to Chat Completions format for Databricks.
///
/// Databricks uses the standard Chat Completions API which expects:
/// - `messages` array instead of `instructions` + `input`
/// - `max_tokens` instead of extended response controls
fn convert_to_chat_format(
    body: &Value,
    model: &str,
    instructions: &str,
    input: &[ResponseItem],
) -> Result<Value, ApiError> {
    use serde_json::json;

    // Build messages array from instructions and input
    let mut messages = Vec::new();

    // Add system message from instructions
    if !instructions.is_empty() {
        messages.push(json!({
            "role": "system",
            "content": instructions
        }));
    }

    // Convert ResponseItems to chat messages
    for item in input {
        match item {
            ResponseItem::Message { role, content, .. } => {
                // Extract text content from content blocks
                use jarvis_protocol::models::ContentItem;
                let text_content: String = content
                    .iter()
                    .filter_map(|block| match block {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                            Some(text.clone())
                        }
                        ContentItem::InputImage { .. } => None, // Skip images for now
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !text_content.is_empty() {
                    messages.push(json!({
                        "role": role,
                        "content": text_content
                    }));
                }
            }
            ResponseItem::FunctionCall { name, arguments, .. } => {
                // Convert function call to assistant message with tool call
                messages.push(json!({
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": arguments
                        }
                    }]
                }));
            }
            ResponseItem::FunctionCallOutput { output, .. } => {
                // Convert function return to tool message
                // FunctionCallOutputPayload has: content, content_items, success
                let output_text = if output.success.unwrap_or(true) {
                    output.content.clone()
                } else {
                    format!("Error: {}", output.content)
                };
                messages.push(json!({
                    "role": "user",
                    "content": output_text
                }));
            }
            _ => {
                // For other types (Reasoning, WebSearch, etc.), convert to user message
                // This is a simplified conversion - you may want to handle these differently
                if let Some(content) = extract_content_from_item(item) {
                    messages.push(json!({
                        "role": "user",
                        "content": content
                    }));
                }
            }
        }
    }

    // Build Chat Completions format payload
    let mut chat_body = json!({
        "model": model,
        "messages": messages,
        "stream": body.get("stream").and_then(|v| v.as_bool()).unwrap_or(true),
        "max_tokens": 4096,  // Default, can be made configurable
    });

    // Note: Databricks has strict tool format requirements
    // For now, we skip tools to ensure basic chat works
    // TODO: Transform tools to Databricks-compatible format if needed

    // Preserve temperature if present
    if let Some(temp) = body.get("temperature") {
        chat_body["temperature"] = temp.clone();
    }

    Ok(chat_body)
}

/// Extracts text content from various ResponseItem types
fn extract_content_from_item(item: &ResponseItem) -> Option<String> {
    use jarvis_protocol::models::ContentItem;
    match item {
        ResponseItem::Message { content, .. } => {
            let text: String = content
                .iter()
                .filter_map(|block| match block {
                    ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                        Some(text.as_str())
                    }
                    ContentItem::InputImage { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            if text.is_empty() { None } else { Some(text) }
        }
        ResponseItem::Reasoning { content, .. } => {
            // Extract text from reasoning content if available
            content.as_ref().and_then(|contents| {
                let text: String = contents
                    .iter()
                    .filter_map(|c| match c {
                        jarvis_protocol::models::ReasoningItemContent::Text { text } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.is_empty() { None } else { Some(text) }
            })
        }
        ResponseItem::WebSearchCall { action, .. } => {
            action.as_ref().and_then(|a| match a {
                jarvis_protocol::models::WebSearchAction::Search { query, queries, .. } => {
                    if let Some(q) = query {
                        Some(format!("Web search: {}", q))
                    } else if let Some(qs) = queries {
                        Some(format!("Web searches: {}", qs.join(", ")))
                    } else {
                        None
                    }
                }
                _ => None,
            })
        }
        ResponseItem::LocalShellCall { action, .. } => match action {
            jarvis_protocol::models::LocalShellAction::Exec(exec) => {
                Some(format!("Shell: {}", exec.command.join(" ")))
            }
        },
        ResponseItem::CustomToolCall { name, input, .. } => {
            Some(format!("Tool call: {} with input: {}", name, input))
        }
        ResponseItem::CustomToolCallOutput { output, .. } => Some(output.clone()),
        _ => None,
    }
}

fn attach_item_ids(payload_json: &mut Value, original_items: &[ResponseItem]) {
    let Some(input_value) = payload_json.get_mut("input") else {
        return;
    };
    let Value::Array(items) = input_value else {
        return;
    };

    for (value, item) in items.iter_mut().zip(original_items.iter()) {
        if let ResponseItem::Reasoning { id, .. }
        | ResponseItem::Message { id: Some(id), .. }
        | ResponseItem::WebSearchCall { id: Some(id), .. }
        | ResponseItem::FunctionCall { id: Some(id), .. }
        | ResponseItem::LocalShellCall { id: Some(id), .. }
        | ResponseItem::CustomToolCall { id: Some(id), .. } = item
        {
            if id.is_empty() {
                continue;
            }

            if let Some(obj) = value.as_object_mut() {
                obj.insert("id".to_string(), Value::String(id.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::RetryConfig;
    use jarvis_protocol::protocol::SubAgentSource;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;
    use std::time::Duration;

    fn provider(name: &str, base_url: &str) -> Provider {
        Provider {
            name: name.to_string(),
            base_url: base_url.to_string(),
            query_params: None,
            headers: HeaderMap::new(),
            retry: RetryConfig {
                max_attempts: 1,
                base_delay: Duration::from_millis(50),
                retry_429: false,
                retry_5xx: true,
                retry_transport: true,
            },
            stream_idle_timeout: Duration::from_secs(5),
        }
    }

    #[test]
    fn azure_default_store_attaches_ids_and_headers() {
        let provider = provider("azure", "https://example.openai.azure.com/v1");
        let input = vec![
            ResponseItem::Message {
                id: Some("m1".into()),
                role: "assistant".into(),
                content: Vec::new(),
                end_turn: None,
                phase: None,
            },
            ResponseItem::Message {
                id: None,
                role: "assistant".into(),
                content: Vec::new(),
                end_turn: None,
                phase: None,
            },
        ];

        let request = ResponsesRequestBuilder::new("gpt-test", "inst", &input)
            .conversation(Some("conv-1".into()))
            .session_source(Some(SessionSource::SubAgent(SubAgentSource::Review)))
            .build(&provider)
            .expect("request");

        assert_eq!(request.body.get("store"), Some(&Value::Bool(true)));

        let ids: Vec<Option<String>> = request
            .body
            .get("input")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .map(|item| item.get("id").and_then(|v| v.as_str().map(str::to_string)))
            .collect();
        assert_eq!(ids, vec![Some("m1".to_string()), None]);

        assert_eq!(
            request.headers.get("session_id"),
            Some(&HeaderValue::from_static("conv-1"))
        );
        assert_eq!(
            request.headers.get("x-openai-subagent"),
            Some(&HeaderValue::from_static("review"))
        );
    }
}
