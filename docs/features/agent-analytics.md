# Analytics de Agentes

Este documento descreve como usar as funcionalidades de analytics para entender padrões de operações dos agentes.

## Visão Geral

O sistema de analytics permite analisar:

- Padrões de uso de ferramentas
- Taxas de sucesso e falha
- Durações de operações
- Cadeias de ferramentas
- Decisões de aprovação

## Queries Disponíveis

### Estatísticas de Uso de Ferramentas

Obtenha contagens de uso por ferramenta em uma thread:

```rust
use JARVIS_state::StateRuntime;

let stats = state_runtime.tool_usage_stats(thread_id).await?;
// Retorna: HashMap<String, u64>
// Exemplo: {"shell" => 10, "file_read" => 5, "file_write" => 3}
```

### Taxas de Sucesso

Calcule taxas de sucesso por ferramenta:

```rust
let success_rates = state_runtime.tool_success_rates(Some(thread_id)).await?;
// Retorna: HashMap<String, f64>
// Exemplo: {"shell" => 0.9, "file_read" => 1.0, "file_write" => 0.8}
// Valores entre 0.0 e 1.0
```

Para todas as threads:

```rust
let success_rates = state_runtime.tool_success_rates(None).await?;
```

### Durações Médias

Obtenha durações médias de execução por ferramenta:

```rust
let avg_durations = state_runtime.tool_avg_durations(Some(thread_id)).await?;
// Retorna: HashMap<String, f64>
// Exemplo: {"shell" => 150.5, "file_read" => 25.0, "file_write" => 45.2}
// Valores em milissegundos
```

### Padrões de Cadeias de Ferramentas

Identifique sequências comuns de ferramentas:

```rust
let chains = state_runtime.tool_chain_patterns(thread_id, 3).await?;
// Retorna: Vec<Vec<String>>
// Exemplo: [
//   ["shell", "file_read", "file_write"],
//   ["file_read", "file_write", "shell"]
// ]
// min_chain_length: comprimento mínimo da cadeia (ex: 3)
```

### Estatísticas de Decisões

Analise padrões de aprovação/negação:

```rust
let decision_stats = state_runtime.decision_stats(Some(thread_id)).await?;
// Retorna: HashMap<String, u64>
// Exemplo: {"approved" => 50, "denied" => 5}
```

## Exemplos de Uso

### Identificar Ferramentas Problemáticas

```rust
// Encontrar ferramentas com baixa taxa de sucesso
let success_rates = state_runtime.tool_success_rates(None).await?;
for (tool, rate) in success_rates {
    if rate < 0.8 {
        println!("⚠️  {tool}: {:.1}% success rate", rate * 100.0);
    }
}
```

### Analisar Performance

```rust
// Encontrar ferramentas mais lentas
let avg_durations = state_runtime.tool_avg_durations(None).await?;
let mut sorted: Vec<_> = avg_durations.into_iter().collect();
sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

println!("Top 5 slowest tools:");
for (tool, duration) in sorted.iter().take(5) {
    println!("  {tool}: {duration:.2}ms");
}
```

### Detectar Padrões Comuns

```rust
// Encontrar cadeias de ferramentas mais comuns
let chains = state_runtime.tool_chain_patterns(thread_id, 2).await?;
let mut chain_counts: std::collections::HashMap<String, u64> = 
    std::collections::HashMap::new();

for chain in chains {
    let key = chain.join(" -> ");
    *chain_counts.entry(key).or_insert(0) += 1;
}

let mut sorted: Vec<_> = chain_counts.into_iter().collect();
sorted.sort_by(|a, b| b.1.cmp(&a.1));

println!("Most common tool chains:");
for (chain, count) in sorted.iter().take(10) {
    println!("  {chain}: {count} times");
}
```

## Integração com Métricas

Combine analytics com métricas em tempo real:

```rust
use codex_otel::OtelManager;

let manager = OtelManager::new(/* ... */);

// Registrar métricas baseadas em analytics
let success_rates = state_runtime.tool_success_rates(None).await?;
for (tool, rate) in success_rates {
    manager.record_operation_success_rate(
        &tool,
        rate > 0.8,
        1,
    );
}
```

## Visualização

Use as queries analíticas para alimentar dashboards:

1. **Grafana**: Use queries SQL ou exporte dados para Prometheus
2. **Custom Dashboards**: Crie visualizações usando as estruturas retornadas
3. **Relatórios**: Gere relatórios periódicos usando as estatísticas

## Performance

As queries são otimizadas com índices:

- `idx_tool_operations_thread_id`: Queries por thread
- `idx_tool_operations_tool_name`: Queries por ferramenta
- `idx_tool_operations_created_at`: Ordenação temporal
- `idx_tool_operations_success`: Filtros de sucesso/falha

Para grandes volumes de dados, considere:

- Limitar resultados com `limit` nas queries
- Usar agregações no banco de dados
- Implementar cache para queries frequentes
