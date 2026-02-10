use crate::harness::build_metrics_with_defaults;
use crate::harness::find_metric;
use jarvis_protocol::protocol::SessionSource;
use jarvis_protocol::ThreadId;
use jarvis_otel::OtelManager;
use jarvis_otel::metrics::MetricsClient;
use opentelemetry_sdk::metrics::InMemoryMetricExporter;
use pretty_assertions::assert_eq;
use std::time::Duration;

struct Harness {
    thread_id: ThreadId,
    metrics: MetricsClient,
    exporter: InMemoryMetricExporter,
}

impl Harness {
    fn new() -> Self {
        let (metrics, exporter) = build_metrics_with_defaults(&[]).unwrap();
        Self {
            thread_id: ThreadId::new_v4(),
            metrics,
            exporter,
        }
    }
}

#[tokio::test]
async fn record_tool_pattern() {
    let harness = Harness::new();
    let manager = OtelManager::new(
        harness.thread_id,
        "gpt-4",
        "gpt-4",
        None,
        None,
        None,
        false,
        "test".to_string(),
        SessionSource::Cli,
    )
    .with_metrics(harness.metrics.clone());

    manager.record_tool_pattern("shell", 1);
    manager.record_tool_pattern("file_read", 2);
    manager.record_tool_pattern("shell", 3);

    harness.metrics.shutdown().unwrap();
    
    // Get metrics from exporter
    let metrics = harness.exporter.get_finished_metrics().unwrap();
    let Some(latest) = metrics.into_iter().last() else {
        panic!("no metrics exported");
    };

    // Verify metrics were recorded
    let tool_pattern_metric = find_metric(&latest, "Jarvis.agent.tool_pattern");
    assert!(tool_pattern_metric.is_some(), "tool_pattern metric should exist");
}

#[tokio::test]
async fn record_operation_success_rate() {
    let harness = Harness::new();
    let manager = OtelManager::new(
        harness.thread_id,
        "gpt-4",
        "gpt-4",
        None,
        None,
        None,
        false,
        "test".to_string(),
        SessionSource::Cli,
    )
    .with_metrics(harness.metrics.clone());

    manager.record_operation_success_rate("file_operation", true, 1);
    manager.record_operation_success_rate("file_operation", false, 1);
    manager.record_operation_success_rate("api_call", true, 2);

    harness.metrics.shutdown().unwrap();
    
    let metrics = harness.exporter.get_finished_metrics().unwrap();
    let Some(latest) = metrics.into_iter().last() else {
        panic!("no metrics exported");
    };

    let success_rate_metric = find_metric(&latest, "Jarvis.agent.operation.success_rate");
    assert!(
        success_rate_metric.is_some(),
        "operation_success_rate metric should exist"
    );
}

#[tokio::test]
async fn record_decision() {
    let harness = Harness::new();
    let manager = OtelManager::new(
        harness.thread_id,
        "gpt-4",
        "gpt-4",
        None,
        None,
        None,
        false,
        "test".to_string(),
        SessionSource::Cli,
    )
    .with_metrics(harness.metrics.clone());

    manager.record_decision("approved", "user", 1);
    manager.record_decision("denied", "user", 1);
    manager.record_decision("approved", "config", 2);

    harness.metrics.shutdown().unwrap();
    
    let metrics = harness.exporter.get_finished_metrics().unwrap();
    let Some(latest) = metrics.into_iter().last() else {
        panic!("no metrics exported");
    };

    let decision_metric = find_metric(&latest, "Jarvis.agent.decision");
    assert!(decision_metric.is_some(), "decision metric should exist");
}

#[tokio::test]
async fn record_conversation_duration() {
    let harness = Harness::new();
    let manager = OtelManager::new(
        harness.thread_id,
        "gpt-4",
        "gpt-4",
        None,
        None,
        None,
        false,
        "test".to_string(),
        SessionSource::Cli,
    )
    .with_metrics(harness.metrics.clone());

    manager.record_conversation_duration(Duration::from_secs(30));
    manager.record_conversation_duration(Duration::from_millis(500));

    harness.metrics.shutdown().unwrap();
    
    let metrics = harness.exporter.get_finished_metrics().unwrap();
    let Some(latest) = metrics.into_iter().last() else {
        panic!("no metrics exported");
    };

    let duration_metric = find_metric(&latest, "Jarvis.agent.conversation.duration_ms");
    assert!(
        duration_metric.is_some(),
        "conversation_duration metric should exist"
    );
}

#[tokio::test]
async fn record_tool_chain_length() {
    let harness = Harness::new();
    let manager = OtelManager::new(
        harness.thread_id,
        "gpt-4",
        "gpt-4",
        None,
        None,
        None,
        false,
        "test".to_string(),
        SessionSource::Cli,
    )
    .with_metrics(harness.metrics.clone());

    manager.record_tool_chain_length(3);
    manager.record_tool_chain_length(5);
    manager.record_tool_chain_length(1);

    harness.metrics.shutdown().unwrap();
    
    let metrics = harness.exporter.get_finished_metrics().unwrap();
    let Some(latest) = metrics.into_iter().last() else {
        panic!("no metrics exported");
    };

    let chain_length_metric = find_metric(&latest, "Jarvis.agent.tool.chain_length");
    assert!(
        chain_length_metric.is_some(),
        "tool_chain_length metric should exist"
    );
}

#[tokio::test]
async fn agent_metrics_without_metrics_client() {
    // Test that metrics methods don't panic when metrics client is not configured
    let manager = OtelManager::new(
        jarvis_protocol::ThreadId::new_v4(),
        "gpt-4",
        "gpt-4",
        None,
        None,
        None,
        false,
        "test".to_string(),
        SessionSource::Cli,
    );

    // These should not panic even without metrics
    manager.record_tool_pattern("shell", 1);
    manager.record_operation_success_rate("file_operation", true, 1);
    manager.record_decision("approved", "user", 1);
    manager.record_conversation_duration(Duration::from_secs(10));
    manager.record_tool_chain_length(3);
}
