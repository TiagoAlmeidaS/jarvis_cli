# Observabilidade e Monitoramento

Este documento descreve o sistema de observabilidade implementado no Jarvis CLI, incluindo métricas, traces, logs e persistência de operações.

## Visão Geral

O sistema de observabilidade é baseado em OpenTelemetry e fornece:

- **Métricas**: Contadores, histogramas e métricas customizadas de agentes
- **Traces**: Rastreamento distribuído de operações
- **Logs**: Logs estruturados com contexto
- **Persistência**: Armazenamento de operações de ferramentas para análise

## Configuração

### Métricas

As métricas são exportadas via OTLP (OpenTelemetry Protocol) para um collector que pode converter para Prometheus:

```rust
use codex_otel::config::{OtelExporter, OtelHttpProtocol, OtelSettings};
use codex_otel::otel_provider::OtelProvider;

let settings = OtelSettings {
    environment: "production".to_string(),
    service_name: "jarvis-cli".to_string(),
    service_version: env!("CARGO_PKG_VERSION").to_string(),
    jarvis_home: std::path::PathBuf::from("/tmp"),
    exporter: OtelExporter::OtlpHttp {
        endpoint: "http://otel-collector:4318/v1/logs".to_string(),
        headers: std::collections::HashMap::new(),
        protocol: OtelHttpProtocol::Json,
        tls: None,
    },
    trace_exporter: OtelExporter::OtlpHttp {
        endpoint: "http://otel-collector:4318/v1/traces".to_string(),
        headers: std::collections::HashMap::new(),
        protocol: OtelHttpProtocol::Json,
        tls: None,
    },
    metrics_exporter: OtelExporter::OtlpHttp {
        endpoint: "http://otel-collector:4318/v1/metrics".to_string(),
        headers: std::collections::HashMap::new(),
        protocol: OtelHttpProtocol::Json,
        tls: None,
    },
    runtime_metrics: true,
};

if let Some(provider) = OtelProvider::from(&settings)? {
    let registry = tracing_subscriber::registry()
        .with(provider.logger_layer())
        .with(provider.tracing_layer());
    registry.init();
}
```

### Prometheus e Grafana

Para visualizar métricas no Grafana:

1. Configure um OpenTelemetry Collector com exporter Prometheus
2. Configure Prometheus para fazer scraping do collector
3. Configure Grafana para usar Prometheus como fonte de dados

Exemplo de configuração do Collector (`otel-collector-config.yaml`):

```yaml
receivers:
  otlp:
    protocols:
      http:
        endpoint: 0.0.0.0:4318

exporters:
  prometheus:
    endpoint: "0.0.0.0:8889"

service:
  pipelines:
    metrics:
      receivers: [otlp]
      exporters: [prometheus]
```

## Métricas de Agentes

O sistema fornece métricas específicas para análise de padrões de operações dos agentes:

### Métricas Disponíveis

- `Jarvis.agent.tool_pattern`: Padrões de uso de ferramentas
- `Jarvis.agent.operation.success_rate`: Taxa de sucesso por tipo de operação
- `Jarvis.agent.decision`: Decisões de aprovação (approved/denied)
- `Jarvis.agent.conversation.duration_ms`: Duração de conversações
- `Jarvis.agent.tool.chain_length`: Comprimento de cadeias de ferramentas

### Uso

```rust
use codex_otel::OtelManager;

let manager = OtelManager::new(/* ... */);

// Registrar padrão de uso de ferramenta
manager.record_tool_pattern("shell", 1);

// Registrar taxa de sucesso
manager.record_operation_success_rate("file_operation", true, 1);

// Registrar decisão
manager.record_decision("approved", "user", 1);

// Registrar duração de conversação
manager.record_conversation_duration(Duration::from_secs(30));

// Registrar comprimento de cadeia de ferramentas
manager.record_tool_chain_length(5);
```

## Persistência de Operações

As operações de ferramentas são automaticamente persistidas no banco de dados SQLite quando `OtelManager` é configurado com `StateRuntime`:

```rust
use codex_otel::OtelManager;
use JARVIS_state::StateRuntime;

let state_runtime = StateRuntime::init(jarvis_home, "provider".to_string(), None).await?;
let manager = OtelManager::new(/* ... */)
    .with_state_runtime(state_runtime.clone());

// Operações são persistidas automaticamente
manager.tool_result(
    "shell",
    "call-123",
    r#"{"command": "ls -la"}"#,
    Duration::from_millis(100),
    true,
    "output",
);
```

## Analytics e Insights

O módulo `state` fornece queries analíticas para análise de padrões:

```rust
use JARVIS_state::StateRuntime;

// Estatísticas de uso de ferramentas
let stats = state_runtime.tool_usage_stats(thread_id).await?;

// Taxas de sucesso por ferramenta
let success_rates = state_runtime.tool_success_rates(Some(thread_id)).await?;

// Durações médias por ferramenta
let avg_durations = state_runtime.tool_avg_durations(Some(thread_id)).await?;

// Padrões de cadeias de ferramentas
let chains = state_runtime.tool_chain_patterns(thread_id, 3).await?;

// Estatísticas de decisões
let decision_stats = state_runtime.decision_stats(Some(thread_id)).await?;
```

## Dashboards Grafana

Exemplos de queries PromQL para dashboards:

### Taxa de Sucesso de Operações

```promql
sum(rate(Jarvis_agent_operation_success_rate{success="true"}[5m])) 
/ 
sum(rate(Jarvis_agent_operation_success_rate[5m]))
```

### Duração Média de Conversações

```promql
avg(Jarvis_agent_conversation_duration_ms)
```

### Top 10 Ferramentas Mais Usadas

```promql
topk(10, sum by (tool) (rate(Jarvis_agent_tool_pattern[5m])))
```

### Comprimento Médio de Cadeias de Ferramentas

```promql
avg(Jarvis_agent_tool_chain_length)
```

## Retenção de Dados

Por padrão, os dados persistidos são mantidos indefinidamente. Para configurar retenção:

```rust
// Implementar política de retenção baseada em timestamp
// Exemplo: deletar operações mais antigas que 30 dias
let cutoff = Utc::now() - chrono::Duration::days(30);
// Adicionar método delete_tool_operations_before() se necessário
```

## Privacidade

Dados sensíveis em `arguments` e `result` são armazenados como estão. Considere implementar redação antes da persistência se necessário:

```rust
// Exemplo de redação (implementar conforme necessário)
fn redact_sensitive_data(data: &str) -> String {
    // Implementar lógica de redação
    data.to_string()
}
```
