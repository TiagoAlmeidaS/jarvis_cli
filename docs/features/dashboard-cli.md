# Dashboard CLI

**Data**: 2026-02-13
**Status**: Implementado
**Modulo**: `jarvis-rs/cli/src/daemon_cmd.rs`

## Overview

Dashboard rico no terminal que apresenta uma visao consolidada do estado do sistema autonomo Jarvis, incluindo metricas, revenue, pipelines, goals, experimentos A/B, e conteudo recente.

## Uso

```bash
# Dashboard padrao (30 dias)
jarvis daemon dashboard

# Dashboard com periodo customizado
jarvis daemon dashboard -d 7

# Versao compacta (sem detalhes de pipelines e conteudo)
jarvis daemon dashboard --compact
```

## Secoes do Dashboard

### 1. Header
- Nome do dashboard com periodo
- Status de saude (HEALTHY / DEGRADED / INACTIVE / AT RISK)

### 2. Metrics Overview
- Views, Clicks, CTR, Revenue, Impressions
- Dados agregados de todas as fontes

### 3. Revenue by Source
- Breakdown por fonte com barras proporcionais
- Percentual de contribuicao de cada fonte

### 4. Pipelines
- Contagem habilitadas/total
- Jobs em execucao, completados e falhados (24h)
- Lista detalhada com status, strategy e schedule (modo normal)

### 5. Goals
- Progress bars ASCII para cada goal ativo
- Status: DONE / OK / RISK
- Prioridade e valores atuais vs target

### 6. A/B Experiments
- Experimentos em execucao com metricas A vs B
- Indicador de lider atual

### 7. Recent Content
- Ultimos 10 conteudos publicados (modo normal)
- Titulo, contagem de palavras, data de publicacao

### 8. Pending Proposals
- Alerta quando ha proposals aguardando revisao

## Health Assessment

| Condicao | Status |
|----------|--------|
| Falhas > Sucessos (24h) | DEGRADED |
| Nenhuma pipeline habilitada | INACTIVE |
| Goals com < 40% progresso | AT RISK |
| Caso contrario | HEALTHY |

## Argumentos

| Flag | Descricao | Default |
|------|-----------|---------|
| `-d`, `--days` | Periodo em dias | 30 |
| `--compact` | Modo compacto | false |

## Exemplo de Output

```
════════════════════════════════════════════════════════════════════════
  JARVIS DAEMON DASHBOARD (30d)   Status: HEALTHY
════════════════════════════════════════════════════════════════════════

  METRICS OVERVIEW
  ────────────────────────────────────────────────────────────────────
  Views: 1234        Clicks: 567         CTR: 4.50%       Revenue: $45.00
  Impressions: 12600

  REVENUE BY SOURCE
  ────────────────────────────────────────────────────────────────────
  adsense         $30.00     ██████████████████████ 67%
  estimated       $15.00     ██████████ 33%

  PIPELINES
  ────────────────────────────────────────────────────────────────────
  3/4 enabled  |  1 running jobs  |  12 completed (24h)  |  0 failed (24h)
  ● SEO Blog Generator        seo_blog        0 3 * * *
  ● Metrics Collector          metrics_collector 0 */6 * * *
  ● Strategy Analyzer          strategy_analyzer 0 2 * * 1

  GOALS
  ────────────────────────────────────────────────────────────────────
  P1 Monthly Revenue $50       30.0/50.0 USD [████████████░░░░░░░░] [OK]
  P2 500 Pageviews/month       234.0/500.0 views [█████████░░░░░░░░░░░] [RISK]

════════════════════════════════════════════════════════════════════════
  Last updated: 2026-02-13 15:30:00 UTC
```

## Arquivos Afetados

- `jarvis-rs/cli/src/daemon_cmd.rs` — Implementacao do subcommand `dashboard`
