pub(crate) const TOOL_CALL_COUNT_METRIC: &str = "Jarvis.tool.call";
pub(crate) const TOOL_CALL_DURATION_METRIC: &str = "Jarvis.tool.call.duration_ms";
pub(crate) const API_CALL_COUNT_METRIC: &str = "Jarvis.api_request";
pub(crate) const API_CALL_DURATION_METRIC: &str = "Jarvis.api_request.duration_ms";
pub(crate) const SSE_EVENT_COUNT_METRIC: &str = "Jarvis.sse_event";
pub(crate) const SSE_EVENT_DURATION_METRIC: &str = "Jarvis.sse_event.duration_ms";
pub(crate) const WEBSOCKET_REQUEST_COUNT_METRIC: &str = "Jarvis.websocket.request";
pub(crate) const WEBSOCKET_REQUEST_DURATION_METRIC: &str = "Jarvis.websocket.request.duration_ms";
pub(crate) const WEBSOCKET_EVENT_COUNT_METRIC: &str = "Jarvis.websocket.event";
pub(crate) const WEBSOCKET_EVENT_DURATION_METRIC: &str = "Jarvis.websocket.event.duration_ms";
pub(crate) const RESPONSES_API_OVERHEAD_DURATION_METRIC: &str =
    "Jarvis.responses_api_overhead.duration_ms";
pub(crate) const RESPONSES_API_INFERENCE_TIME_DURATION_METRIC: &str =
    "Jarvis.responses_api_inference_time.duration_ms";

// Agent-specific metrics
pub(crate) const AGENT_TOOL_PATTERN_METRIC: &str = "Jarvis.agent.tool_pattern";
pub(crate) const AGENT_OPERATION_SUCCESS_RATE_METRIC: &str = "Jarvis.agent.operation.success_rate";
pub(crate) const AGENT_DECISION_METRIC: &str = "Jarvis.agent.decision";
pub(crate) const AGENT_CONVERSATION_DURATION_METRIC: &str = "Jarvis.agent.conversation.duration_ms";
pub(crate) const AGENT_TOOL_CHAIN_LENGTH_METRIC: &str = "Jarvis.agent.tool.chain_length";
