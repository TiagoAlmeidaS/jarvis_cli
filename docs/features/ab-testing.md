# A/B Testing de Titulos SEO

**Status**: Implementado  
**Data**: 2026-02-13  
**Roadmap**: Fase 3, item 3.3

## Visao Geral

O sistema de A/B testing permite que o daemon teste automaticamente variantes de titulos
SEO em artigos publicados, meça a performance via CTR (click-through rate) do Google
Search Console, e promova o vencedor.

## Arquitetura

```
Pipeline ab_tester (executa periodicamente)
        |
        v
  Fase 1: Avaliar experimentos maduros
        |-- Buscar CTR do Search Console
        |-- Comparar metricas A vs B
        |-- Se diff >= threshold: declarar vencedor
        |
        v
  Fase 2: Criar novos experimentos
        |-- Encontrar conteudo publicado sem experimento ativo
        |-- Gerar titulo alternativo via LLM
        |-- Criar registro em daemon_experiments
```

## Componentes

### Models (`daemon-common/src/models.rs`)

| Tipo | Descricao |
|------|-----------|
| `ExperimentStatus` | `running`, `completed`, `cancelled` |
| `ExperimentType` | `title`, `meta_description`, `headline`, `custom` |
| `DaemonExperiment` | Registro completo de um experimento |
| `CreateExperiment` | Input para criar um novo experimento |
| `ExperimentFilter` | Filtros para listagem |

### Tabela `daemon_experiments`

| Coluna | Tipo | Descricao |
|--------|------|-----------|
| `id` | TEXT PK | UUID |
| `content_id` | TEXT | Referencia ao daemon_content |
| `pipeline_id` | TEXT | Pipeline dono do conteudo |
| `experiment_type` | TEXT | Tipo do teste (title, etc.) |
| `status` | TEXT | running / completed / cancelled |
| `variant_a` | TEXT | Valor original |
| `variant_b` | TEXT | Valor challenger |
| `active_variant` | TEXT | Qual variante esta live ("a" ou "b") |
| `metric` | TEXT | Metrica sendo medida (default: "ctr") |
| `metric_a` | REAL | Valor acumulado para variante A |
| `metric_b` | REAL | Valor acumulado para variante B |
| `winner` | TEXT | Vencedor ("a" ou "b", null se running) |
| `min_duration_days` | INT | Minimo de dias antes de declarar vencedor |
| `created_at` | INT | Timestamp de criacao |
| `completed_at` | INT | Timestamp de conclusao |

### DB Methods (`daemon-common/src/db.rs`)

- `create_experiment()` — Cria novo experimento
- `get_experiment()` — Busca por ID
- `list_experiments()` — Lista com filtros
- `switch_experiment_variant()` — Troca variante ativa
- `update_experiment_metrics()` — Atualiza metricas A/B
- `complete_experiment()` — Finaliza com vencedor
- `cancel_experiment()` — Cancela experimento
- `list_mature_experiments()` — Lista experimentos que ja passaram do min_duration

### Pipeline (`daemon/src/pipelines/ab_tester.rs`)

Strategy: `ab_tester`

#### Configuracao

```json
{
  "max_concurrent_experiments": 3,
  "min_duration_days": 7,
  "min_impressions": 100,
  "min_ctr_diff_pct": 0.5,
  "target_pipeline_id": "seo-concursos",
  "wordpress_base_url": "https://seu-blog.com"
}
```

| Campo | Default | Descricao |
|-------|---------|-----------|
| `max_concurrent_experiments` | 3 | Max experimentos simultaneos |
| `min_duration_days` | 7 | Dias minimos antes de decidir |
| `min_impressions` | 100 | Impressoes minimas para significancia |
| `min_ctr_diff_pct` | 0.5 | Diferenca minima em CTR (pp) para declarar vencedor |
| `target_pipeline_id` | null | Pipeline alvo (null = todos) |

## Como Usar

1. Crie uma pipeline do tipo `ab_tester`:

```bash
jarvis daemon pipeline create \
  --id ab-seo \
  --name "A/B SEO Titles" \
  --strategy ab_tester \
  --schedule "0 6 * * *" \
  --config '{"target_pipeline_id":"seo-concursos","min_duration_days":7}'
```

2. O pipeline roda automaticamente no schedule configurado.

3. Monitore experimentos:
   - Os logs do pipeline mostram criacao e conclusao de experimentos
   - Experimentos finalizados tem o vencedor registrado no DB

## Testes

- `ab_tester::tests::parse_config_defaults`
- `ab_tester::tests::parse_config_custom`
- `ab_tester::tests::strategy_name`
- `ab_tester::tests::validate_config_accepts_empty`
- `ab_tester::tests::validate_config_rejects_bad_json`
- `ab_tester::tests::experiment_crud_roundtrip`
- `ab_tester::tests::cancel_experiment`
- `ab_tester::tests::list_mature_experiments_respects_duration`
