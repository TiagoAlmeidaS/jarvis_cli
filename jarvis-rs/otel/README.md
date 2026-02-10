# jarvis-otel

`jarvis-otel` is the OpenTelemetry integration crate for jarvis. It provides:

- Trace/log/metrics exporters and tracing subscriber layers (`codex_otel::otel_provider`).
- A structured event helper (`codex_otel::OtelManager`).
- OpenTelemetry metrics support via OTLP exporters (`codex_otel::metrics`).
- A metrics facade on `OtelManager` so tracing + metrics share metadata.

## Tracing and logs

Create an OTEL provider from `OtelSettings`. The provider also configures
metrics (when enabled), then attach its layers to your `tracing_subscriber`
registry:

```rust
use codex_otel::config::OtelExporter;
use codex_otel::config::OtelHttpProtocol;
use codex_otel::config::OtelSettings;
use codex_otel::otel_provider::OtelProvider;
use tracing_subscriber::prelude::*;

let settings = OtelSettings {
    environment: "dev".to_string(),
    service_name: "jarvis-cli".to_string(),
    service_version: env!("CARGO_PKG_VERSION").to_string(),
    jarvis_home: std::path::PathBuf::from("/tmp"),
    exporter: OtelExporter::OtlpHttp {
        endpoint: "https://otlp.example.com".to_string(),
        headers: std::collections::HashMap::new(),
        protocol: OtelHttpProtocol::Binary,
        tls: None,
    },
    trace_exporter: OtelExporter::OtlpHttp {
        endpoint: "https://otlp.example.com".to_string(),
        headers: std::collections::HashMap::new(),
        protocol: OtelHttpProtocol::Binary,
        tls: None,
    },
    metrics_exporter: OtelExporter::None,
};

if let Some(provider) = OtelProvider::from(&settings)? {
    let registry = tracing_subscriber::registry()
        .with(provider.logger_layer())
        .with(provider.tracing_layer());
    registry.init();
}
```

## OtelManager (events)

`OtelManager` adds consistent metadata to tracing events and helps record
jarvis-specific events.

```rust
use codex_otel::OtelManager;

let manager = OtelManager::new(
    conversation_id,
    model,
    slug,
    account_id,
    account_email,
    auth_mode,
    log_user_prompts,
    terminal_type,
    session_source,
);

manager.user_prompt(&prompt_items);
```

## Metrics (OTLP or in-memory)

Modes:

- OTLP: exports metrics via the OpenTelemetry OTLP exporter (HTTP or gRPC).
- In-memory: records via `opentelemetry_sdk::metrics::InMemoryMetricExporter` for tests/assertions; call `shutdown()` to flush.

`jarvis-otel` also provides `OtelExporter::Statsig`, a shorthand for exporting OTLP/HTTP JSON metrics
to Statsig using jarvis-internal defaults.

Statsig ingestion (OTLP/HTTP JSON) example:

```rust
use codex_otel::config::{OtelExporter, OtelHttpProtocol};

let metrics = MetricsClient::new(MetricsConfig::otlp(
    "dev",
    "jarvis-cli",
    env!("CARGO_PKG_VERSION"),
    OtelExporter::OtlpHttp {
        endpoint: "https://api.statsig.com/otlp".to_string(),
        headers: std::collections::HashMap::from([(
            "statsig-api-key".to_string(),
            std::env::var("STATSIG_SERVER_SDK_SECRET")?,
        )]),
        protocol: OtelHttpProtocol::Json,
        tls: None,
    },
))?;

metrics.counter("jarvis.session_started", 1, &[("source", "tui")])?;
metrics.histogram("jarvis.request_latency", 83, &[("route", "chat")])?;
```

In-memory (tests):

```rust
let exporter = InMemoryMetricExporter::default();
let metrics = MetricsClient::new(MetricsConfig::in_memory(
    "test",
    "jarvis-cli",
    env!("CARGO_PKG_VERSION"),
    exporter.clone(),
))?;
metrics.counter("jarvis.turns", 1, &[("model", "gpt-5.1")])?;
metrics.shutdown()?; // flushes in-memory exporter
```

## Prometheus Export

To export metrics to Prometheus (for Grafana visualization), configure an OTLP exporter
pointing to an OpenTelemetry Collector that converts OTLP to Prometheus format:

```rust
use codex_otel::config::{OtelExporter, OtelHttpProtocol};

let settings = OtelSettings {
    // ... other settings ...
    metrics_exporter: OtelExporter::OtlpHttp {
        endpoint: "http://otel-collector:4318/v1/metrics".to_string(),
        headers: std::collections::HashMap::new(),
        protocol: OtelHttpProtocol::Json,
        tls: None,
    },
    // ... other settings ...
};
```

The OpenTelemetry Collector should be configured with a Prometheus exporter to expose
metrics at `/metrics` endpoint for Prometheus scraping.

## Agent Metrics

The crate provides helper methods for recording agent-specific metrics:

```rust
use codex_otel::OtelManager;

let manager = OtelManager::new(/* ... */);

// Record tool pattern usage
manager.record_tool_pattern("shell", 1);

// Record operation success rate
manager.record_operation_success_rate("file_operation", true, 1);

// Record decision metrics
manager.record_decision("approved", "user", 1);

// Record conversation duration
manager.record_conversation_duration(Duration::from_secs(30));

// Record tool chain length
manager.record_tool_chain_length(5);
```

## Tool Operations Persistence

Tool operations can be persisted to the state database for analytics:

```rust
use codex_otel::OtelManager;
use JARVIS_state::StateRuntime;

let state_runtime = StateRuntime::init(/* ... */).await?;
let manager = OtelManager::new(/* ... */)
    .with_state_runtime(state_runtime);

// Tool operations will be automatically persisted when tool_result() is called
manager.tool_result(
    "shell",
    "call-123",
    r#"{"command": "ls -la"}"#,
    Duration::from_millis(100),
    true,
    "file1.txt\nfile2.txt",
);
```

Query persisted operations:

```rust
use JARVIS_state::StateRuntime;

let operations = state_runtime.query_tool_operations(thread_id, Some(100)).await?;
let stats = state_runtime.tool_usage_stats(thread_id).await?;
let success_rates = state_runtime.tool_success_rates(Some(thread_id)).await?;
let chains = state_runtime.tool_chain_patterns(thread_id, 3).await?;
```

## Shutdown

- `OtelProvider::shutdown()` stops the OTEL exporter.
- `OtelManager::shutdown_metrics()` flushes and shuts down the metrics provider.

Both are optional because drop performs best-effort shutdown, but calling them
explicitly gives deterministic flushing (or a shutdown error if flushing does
not complete in time).
