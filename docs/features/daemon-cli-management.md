# Daemon CLI Management Commands

**Data**: 2026-02-13
**Status**: Implementado
**Modulo**: `jarvis-rs/cli/src/daemon_cmd.rs`

## Overview

O daemon CLI fornece um conjunto completo de subcommands para inspecionar e controlar o sistema autonomo do Jarvis sem necessidade do daemon estar em execucao. Todos os comandos acessam diretamente o banco SQLite compartilhado.

## Subcommands

### Status
```bash
jarvis daemon status
```
Mostra visao geral: pipelines, jobs em execucao, conteudo publicado recente, proposals pendentes, revenue e goals ativos com progress bars.

### Pipelines
```bash
jarvis daemon pipeline list
jarvis daemon pipeline enable <id>
jarvis daemon pipeline disable <id>
jarvis daemon pipeline config <id>
jarvis daemon pipeline add <config.json>
```
Gerenciamento completo de pipelines: listar, habilitar/desabilitar, visualizar configuracao, e adicionar novas pipelines via JSON.

### Jobs
```bash
jarvis daemon jobs [-p pipeline_id] [-s status] [-n limit]
```
Lista jobs recentes com filtros por pipeline, status (pending/running/completed/failed/cancelled), e limite.

### Content
```bash
jarvis daemon content [-p pipeline_id] [--last-days N] [-n limit]
```
Lista conteudo gerado com filtros por pipeline, periodo, e limite.

### Logs
```bash
jarvis daemon logs [-p pipeline_id] [-j job_id] [-n limit]
```
Visualiza logs do daemon com filtros por pipeline, job, e limite.

### Sources
```bash
jarvis daemon source list <pipeline_id>
jarvis daemon source add <pipeline_id> -t <type> --name <name> <url> [--selector css] [--interval sec]
```
Gerencia fontes de dados (RSS, webpage, API, PDF URL) por pipeline.

### Proposals
```bash
jarvis daemon proposals list [--all] [-p pipeline_id] [-n limit]
jarvis daemon proposals show <id>
jarvis daemon proposals approve <id>
jarvis daemon proposals reject <id> [-r reason]
jarvis daemon proposals expire-stale
```
Visualiza e gerencia proposals geradas pelo strategy analyzer. Suporta partial ID matching, aprovacao/rejeicao, e expiracao de proposals antigas.

### Revenue
```bash
jarvis daemon revenue summary [-d days]
jarvis daemon revenue list [-p pipeline_id] [--last-days N] [-n limit]
jarvis daemon revenue add <pipeline_id> <amount> [-s source] [--currency code] [--external-id id] [--note text]
```
Visualiza e registra receita: sumario por pipeline e fonte, listagem detalhada, e registro manual.

### Goals
```bash
jarvis daemon goals list [--all] [-p pipeline_id]
jarvis daemon goals add <name> -m <metric> -t <target> [--period monthly] [--unit USD] [-p pipeline_id] [--priority N]
jarvis daemon goals progress
jarvis daemon goals pause <id>
jarvis daemon goals resume <id>
jarvis daemon goals archive <id>
```
Gerenciamento de metas: listar, adicionar, visualizar progresso (com barras ASCII), pausar/retomar/arquivar.

### Experiments (Novo)
```bash
jarvis daemon experiments list [-p pipeline_id] [--all] [-n limit]
jarvis daemon experiments show <id>
jarvis daemon experiments cancel <id>
```
Visualiza e gerencia experimentos A/B:
- **list**: Lista experimentos (por padrao apenas running). Mostra tipo, status, metrica, valores A/B, e vencedor.
- **show**: Detalhes completos de um experimento, incluindo variantes, performance, e duracao.
- **cancel**: Cancela um experimento em execucao. O variante ativo permanece.

### Metrics (Novo)
```bash
jarvis daemon metrics summary [-d days]
jarvis daemon metrics content <content_id>
```
Visualizacao de metricas coletadas:
- **summary**: Metricas agregadas (views, clicks, impressions, CTR, revenue) por periodo, com breakdown por fonte (WordPress, Search Console, AdSense, Analytics 4).
- **content**: Metricas de um conteudo especifico (views, clicks, impressions, CTR, revenue).

### Health (Novo)
```bash
jarvis daemon health
```
Diagnostico completo do sistema:
- **Status geral**: HEALTHY / DEGRADED / INACTIVE / AT RISK
- **Sistema**: Pipelines habilitadas, jobs em execucao, taxa de sucesso/falha (24h)
- **Inteligencia**: Proposals pendentes, goals ativos (com risco), experimentos A/B
- **Revenue**: Total 30d com breakdown por fonte
- **Data Sources**: Disponibilidade dos data sources configurados
- **Strategies**: Lista de strategies registradas

## Health Assessment Logic

| Condicao | Status |
|----------|--------|
| Falhas > Sucessos (24h) | DEGRADED |
| Nenhuma pipeline habilitada | INACTIVE |
| Goals com < 40% progresso | AT RISK |
| Caso contrario | HEALTHY |

## Arquivos Afetados

- `jarvis-rs/cli/src/daemon_cmd.rs` — Todos os subcommands

## Dependencias

- `jarvis-daemon-common` — Models, DaemonDb, filtros
- `clap` — Parsing de argumentos
- `owo-colors` — Coloracao do output
- `chrono` — Formatacao de timestamps
