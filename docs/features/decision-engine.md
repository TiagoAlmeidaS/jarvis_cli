# Decision Engine Local

**Status**: Implementado e Integrado  
**Data**: 2026-02-13  
**Roadmap**: Fase 3, item 3.5

## Visao Geral

O Decision Engine e uma camada de inteligencia local que roda **antes** e **depois** do
LLM no strategy_analyzer. Ele da ao daemon um "cerebro" determinístico que:

1. **Pre-filtra**: Decide se vale a pena chamar o LLM (custo vs benefício)
2. **Gera propostas urgentes**: Regras hard-coded para situacoes criticas
3. **Valida**: Bloqueia propostas perigosas do LLM
4. **Ajusta scores**: Combina confianca do LLM com analise de goals

## Arquivos

- `daemon/src/decision_engine.rs` — Motor de regras
- `daemon/src/pipelines/strategy_analyzer.rs` — Integracao no pipeline

## Integracao no Strategy Analyzer

O `StrategyAnalyzerPipeline.execute()` agora segue este fluxo:

1. **Gather data** — Coleta metricas do banco
2. **Build SystemSnapshot** — Cria snapshot com publicacoes, custos, receita, goals
3. **Pre-analyze (DecisionEngine)** — Regras locais decidem se LLM e necessario
4. **Create rule proposals** — Propostas urgentes do engine sao salvas direto no DB
5. **Call LLM** (se `should_call_llm=true`) — Analise profunda
6. **Validate proposals** — Cada proposta do LLM passa por `validate_proposal()`
7. **Adjust confidence** — `adjust_confidence_for_goals()` ajusta scores
8. **Create proposals** — Propostas validadas sao salvas no DB

### Configuracao via pipeline config

```json
{
  "decision_engine": {
    "cost_alert_threshold": 5.0,
    "min_data_points_for_llm": 1,
    "max_pending_proposals": 10
  }
}
```

## Componentes

### SystemSnapshot

Resumo do estado atual do sistema:
- `total_published`: Conteudo publicado
- `total_llm_cost`: Custo total LLM
- `total_revenue`: Revenue total
- `pending_proposals`: Propostas pendentes
- `goals`: Lista de GoalSnapshot
- `pipeline_count`: Numero de pipelines

### GoalSnapshot

Visao resumida de um goal com helpers:
- `progress_pct()`: Progresso em %
- `gap()`: Gap absoluto para o target
- `is_at_risk()`: Se o goal esta em risco (<40% progresso ou deadline proxima)

### Pre-Analysis Rules

| Regra | Condicao | Acao |
|-------|----------|------|
| Pending Limit | >= max_pending | Skip tudo (nao chama LLM) |
| No Data | 0 published, 0 goals | Skip (nao chama LLM) |
| Cost Alert | cost > threshold | Gera proposta `change_model` |
| Goal Urgency | Goal at risk (<40%) | Gera proposta urgente |
| Zero Revenue | $0 com >5 artigos | Gera alerta de monetizacao |

### Validation Rules

| Regra | Condicao | Acao |
|-------|----------|------|
| High Risk + High Cost | risk=high + cost>threshold | Bloqueia |
| Too Many Pipelines | create_pipeline + count>=10 | Bloqueia |
| Low Confidence | confidence < 30% | Bloqueia |

### Confidence Adjustment

O engine ajusta a confianca do LLM baseado em:
- **Boost +5%**: Proposta alinhada com goal at-risk
- **Boost +5%**: Revenue at risk + acao de scale/niche
- **Penalty -3%**: Acao em pipeline sem goal associado

## Configuracao

```json
{
  "cost_alert_threshold": 5.0,
  "min_data_points_for_llm": 1,
  "max_pending_proposals": 10
}
```

## Testes (20 testes)

- `pre_analyze_skips_when_too_many_pending`
- `pre_analyze_skips_when_no_data`
- `pre_analyze_calls_llm_with_data`
- `pre_analyze_generates_cost_alert`
- `pre_analyze_generates_zero_revenue_alert`
- `pre_analyze_generates_goal_urgency`
- `goal_snapshot_progress_and_gap`
- `goal_snapshot_is_at_risk_low_progress`
- `goal_snapshot_not_at_risk_high_progress`
- `goal_snapshot_at_risk_deadline`
- `validate_proposal_blocks_high_risk_when_cost_high`
- `validate_proposal_blocks_low_confidence`
- `validate_proposal_blocks_too_many_pipelines`
- `validate_proposal_allows_good_proposal`
- `adjust_confidence_boosts_for_at_risk_goal`
- `adjust_confidence_clamped_to_1`
- `config_defaults`
- `goal_urgency_cost_limit_exceeded`
- `goal_urgency_content_count`
