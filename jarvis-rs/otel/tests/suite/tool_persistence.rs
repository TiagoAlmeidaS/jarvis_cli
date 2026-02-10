use crate::harness::build_metrics_with_defaults;
use jarvis_protocol::protocol::SessionSource;
use jarvis_protocol::ThreadId;
use jarvis_otel::OtelManager;
use jarvis_otel::metrics::MetricsClient;
use jarvis_state::StateRuntime;
use jarvis_state::ToolOperationBuilder;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;

struct Harness {
    thread_id: ThreadId,
    metrics: MetricsClient,
}

impl Harness {
    fn new() -> Self {
        let (metrics, _exporter) = build_metrics_with_defaults(&[]).unwrap();
        Self {
            thread_id: ThreadId::new_v4(),
            metrics,
        }
    }
}

#[tokio::test]
async fn otel_manager_persists_tool_operation() {
    let harness = Harness::new();
    let jarvis_home = std::env::temp_dir().join(format!(
        "jarvis-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_runtime = Arc::new(
        StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .unwrap(),
    );

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
    .with_metrics(harness.metrics.clone())
    .with_state_runtime(state_runtime.clone());

    // Record a tool result - this should trigger persistence
    manager.tool_result(
        "shell",
        "call-123",
        r#"{"command": "ls -la"}"#,
        Duration::from_millis(100),
        true,
        "file1.txt\nfile2.txt",
    );

    // Wait a bit for async persistence to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify the operation was persisted
    let operations = state_runtime
        .query_tool_operations(harness.thread_id, None)
        .await
        .unwrap();

    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].tool_name, "shell");
    assert_eq!(operations[0].call_id, "call-123");
    assert_eq!(operations[0].arguments, r#"{"command": "ls -la"}"#);
    assert_eq!(
        operations[0].result,
        Some("file1.txt\nfile2.txt".to_string())
    );
    assert!(operations[0].success);
    assert_eq!(operations[0].duration_ms, 100);
}

#[tokio::test]
async fn otel_manager_persists_multiple_operations() {
    let harness = Harness::new();
    let jarvis_home = std::env::temp_dir().join(format!(
        "jarvis-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_runtime = Arc::new(
        StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .unwrap(),
    );

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
    .with_metrics(harness.metrics.clone())
    .with_state_runtime(state_runtime.clone());

    // Record multiple tool results
    manager.tool_result(
        "shell",
        "call-1",
        r#"{"command": "ls"}"#,
        Duration::from_millis(50),
        true,
        "file1.txt",
    );

    manager.tool_result(
        "file_read",
        "call-2",
        r#"{"path": "file1.txt"}"#,
        Duration::from_millis(25),
        true,
        "content",
    );

    manager.tool_result(
        "shell",
        "call-3",
        r#"{"command": "invalid"}"#,
        Duration::from_millis(10),
        false,
        "error",
    );

    // Wait for async persistence
    tokio::time::sleep(Duration::from_millis(200)).await;

    let operations = state_runtime
        .query_tool_operations(harness.thread_id, None)
        .await
        .unwrap();

    assert_eq!(operations.len(), 3);

    // Verify operations are ordered by creation time (most recent first)
    assert_eq!(operations[0].call_id, "call-3");
    assert_eq!(operations[1].call_id, "call-2");
    assert_eq!(operations[2].call_id, "call-1");
}

#[tokio::test]
async fn otel_manager_persistence_without_state_runtime() {
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

    // This should not panic even without state_runtime
    manager.tool_result(
        "shell",
        "call-123",
        r#"{"command": "ls"}"#,
        Duration::from_millis(50),
        true,
        "output",
    );

    // No assertion needed - just verify it doesn't panic
}

#[tokio::test]
async fn query_tool_operations_by_name() {
    let harness = Harness::new();
    let jarvis_home = std::env::temp_dir().join(format!(
        "jarvis-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_runtime = Arc::new(
        StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .unwrap(),
    );

    // Insert operations directly
    for i in 0..5 {
        let op = ToolOperationBuilder::new(
            harness.thread_id,
            format!("shell-call-{i}"),
            "shell".to_string(),
            format!(r#"{{"command": "echo {i}"}}"#),
            true,
            50 + i,
        )
        .build();
        state_runtime.insert_tool_operation(&op).await.unwrap();
    }

    for i in 0..3 {
        let op = ToolOperationBuilder::new(
            harness.thread_id,
            format!("file-call-{i}"),
            "file_read".to_string(),
            format!(r#"{{"path": "file{i}.txt"}}"#),
            true,
            25,
        )
        .build();
        state_runtime.insert_tool_operation(&op).await.unwrap();
    }

    // Query by tool name
    let shell_ops = state_runtime
        .query_tool_operations_by_name("shell", None)
        .await
        .unwrap();

    assert_eq!(shell_ops.len(), 5);
    for op in shell_ops {
        assert_eq!(op.tool_name, "shell");
    }

    let file_ops = state_runtime
        .query_tool_operations_by_name("file_read", None)
        .await
        .unwrap();

    assert_eq!(file_ops.len(), 3);
    for op in file_ops {
        assert_eq!(op.tool_name, "file_read");
    }
}

#[tokio::test]
async fn query_tool_operations_with_limit() {
    let harness = Harness::new();
    let jarvis_home = std::env::temp_dir().join(format!(
        "jarvis-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_runtime = Arc::new(
        StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .unwrap(),
    );

    // Insert 10 operations
    for i in 0..10 {
        let op = ToolOperationBuilder::new(
            harness.thread_id,
            format!("call-{i}"),
            "shell".to_string(),
            format!(r#"{{"command": "echo {i}"}}"#),
            true,
            50,
        )
        .build();
        state_runtime.insert_tool_operation(&op).await.unwrap();
    }

    // Query with limit
    let operations = state_runtime
        .query_tool_operations(harness.thread_id, Some(5))
        .await
        .unwrap();

    assert_eq!(operations.len(), 5);
}
