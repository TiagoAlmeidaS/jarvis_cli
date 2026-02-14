# Prompt Optimizer

**Status**: Implementado  
**Data**: 2026-02-13  
**Roadmap**: Fase 3, item 3.5

## Visao Geral

O Prompt Optimizer analisa automaticamente quais parametros de prompt (niche, tone,
audience, word count) produzem conteudo com melhor performance, e sugere otimizacoes
baseadas nos dados reais.

## Fluxo

1. Identifica pipelines alvo (seo_blog por default)
2. Lista conteudo publicado com metricas
3. Extrai parametros de prompt do config do pipeline
4. Cria hash do prompt para agrupar variantes
5. Scoring: correlaciona CTR, clicks, revenue com parametros
6. LLM analisa performance e sugere mudancas
7. Cria proposals para ajustar pipeline configs

## Arquivos

- `daemon/src/pipelines/prompt_optimizer.rs` — Pipeline principal
- `daemon-common/src/models.rs` — PromptScore, PromptPerformanceSummary, PromptOptimizationSuggestion
- `daemon-common/src/db.rs` — daemon_prompt_scores table + CRUD

## Composite Score

Formula: `CTR * 100 * 0.4 + ln(clicks + 1) * 10 * 0.3 + revenue * 10 * 0.3`

- CTR pesa 40% (principal indicador de qualidade SEO)
- Clicks pesa 30% (log-normalizado para nao dominar)
- Revenue pesa 30% (escalado para valores pequenos)

## Configuracao

```json
{
  "min_content_for_analysis": 5,
  "target_pipeline_ids": [],
  "lookback_days": 30,
  "ctr_weight": 0.4,
  "clicks_weight": 0.3,
  "revenue_weight": 0.3
}
```

## DB Schema

```sql
CREATE TABLE daemon_prompt_scores (
    id, pipeline_id, content_id, prompt_hash,
    params_json, avg_ctr, total_clicks, total_impressions,
    revenue_usd, composite_score, created_at, updated_at
);
```

## Testes (13 testes)

- Config parsing (defaults + custom)
- Strategy name
- Extract params (SEO config + defaults)
- Hash stability + uniqueness
- Suggestion parsing (JSON + empty)
- Config patch building (SEO section + root)
- Validate config
- Prompt score CRUD (DB roundtrip)
