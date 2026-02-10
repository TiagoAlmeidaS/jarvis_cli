use crate::metrics::names::AGENT_CONVERSATION_DURATION_METRIC;
use crate::metrics::names::AGENT_DECISION_METRIC;
use crate::metrics::names::AGENT_OPERATION_SUCCESS_RATE_METRIC;
use crate::metrics::names::AGENT_TOOL_CHAIN_LENGTH_METRIC;
use crate::metrics::names::AGENT_TOOL_PATTERN_METRIC;
use crate::OtelManager;
use std::time::Duration;

/// Helper functions for recording agent-specific metrics.
impl OtelManager {
    /// Record a tool pattern usage metric.
    ///
    /// This metric tracks which tools are being used by agents,
    /// helping identify common patterns and tool preferences.
    pub fn record_tool_pattern(&self, tool_name: &str, count: i64) {
        self.counter(AGENT_TOOL_PATTERN_METRIC, count, &[("tool", tool_name)]);
    }

    /// Record an operation success rate metric.
    ///
    /// This metric tracks the success rate of operations by type,
    /// helping identify problematic operations or tools.
    pub fn record_operation_success_rate(
        &self,
        operation_type: &str,
        success: bool,
        count: i64,
    ) {
        let success_str = if success { "true" } else { "false" };
        self.counter(
            AGENT_OPERATION_SUCCESS_RATE_METRIC,
            count,
            &[("operation_type", operation_type), ("success", success_str)],
        );
    }

    /// Record a decision metric (approved/denied).
    ///
    /// This metric tracks approval decisions made by agents or users,
    /// helping understand decision patterns and approval rates.
    pub fn record_decision(&self, decision_type: &str, source: &str, count: i64) {
        self.counter(
            AGENT_DECISION_METRIC,
            count,
            &[("decision", decision_type), ("source", source)],
        );
    }

    /// Record conversation duration.
    ///
    /// This metric tracks how long conversations take,
    /// helping identify performance patterns and optimization opportunities.
    pub fn record_conversation_duration(&self, duration: Duration) {
        self.record_duration(AGENT_CONVERSATION_DURATION_METRIC, duration, &[]);
    }

    /// Record tool chain length.
    ///
    /// This metric tracks how many tools are chained together in a single operation,
    /// helping identify complex workflows and potential optimization opportunities.
    pub fn record_tool_chain_length(&self, length: i64) {
        self.histogram(AGENT_TOOL_CHAIN_LENGTH_METRIC, length, &[]);
    }
}
