# Pipeline Retry Logic

**Status**: Implementado  
**Data**: 2026-02-13  
**Roadmap**: Fase 2 (resiliencia)

## Visao Geral

Quando um pipeline job falha, o `PipelineRunner` agora verifica se o numero de
tentativas (attempt) e menor que `max_retries` configurado no pipeline. Se for,
cria um novo job com attempt incrementado.

## Fluxo

1. Job falha com erro
2. Runner registra log de erro
3. Verifica `attempt < max_retries`
4. Se sim: cria novo job via `create_job_with_attempt()` com `attempt + 1`
5. Se nao: registra log "All N retries exhausted"

## Arquivos

- `daemon/src/runner.rs` — Logica de retry no `run_job()`
- `daemon-common/src/db.rs` — `create_job_with_attempt()` para criar jobs com attempt especifico

## Configuracao (por pipeline)

| Campo | Tipo | Default | Descricao |
|-------|------|---------|-----------|
| `max_retries` | i32 | 3 | Maximo de tentativas |
| `retry_delay_sec` | i32 | 60 | Delay entre tentativas (usado pelo scheduler) |

## Observacoes

- O retry cria um novo job no estado `pending`, que o scheduler ira executar no proximo tick
- Cada retry incrementa o campo `attempt` para rastreamento
- Logs detalhados de cada tentativa sao persistidos no `daemon_logs`
- Quando todos os retries sao esgotados, um log de erro final e registrado

## Testes

- `test_create_job_with_attempt` — Valida criacao de jobs com attempt custom
