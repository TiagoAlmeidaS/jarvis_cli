# Issues sugeridas: Operação e observabilidade do daemon

Objetivo: operar o daemon e a autonomia em produção de forma sustentável: health check e recuperação, logs e rastreio, métricas e alertas. Apenas observabilidade sobre o que já existe (scheduler, runner, SQLite, pipelines). Referência: [observability](../features/observability.md) (OpenTelemetry, Prometheus/Grafana quando aplicável).

Use este doc para criar as issues no seu repositório (GitHub/GitLab). Cada bloco abaixo é uma issue pronta para copiar.

---

## Issue O1 — Health check e recuperação do daemon

**Título:** `feat(daemon): health check do daemon e recuperação (retry/backoff opcional)`

**Descrição:**

Garantir que o daemon exponha um health check (ex.: endpoint ou comando) e, opcionalmente, comportamento de recuperação (retry/backoff) em falhas transitórias. Útil para deploy em servidor (ex.: homeserver) e orquestradores.

- Health check: endpoint HTTP ou comando `jarvis daemon health` que indique se o daemon está vivo e, se possível, se scheduler/DB estão operantes.
- Opcional: retry/backoff quando um job ou pipeline falha (ex.: API indisponível), para evitar parada total.
- Documentar em deploy-servidor-casa ou DAEMON_QUICK_START.

**Critério de aceite:**

- Health check disponível (endpoint ou CLI); doc indica como usar.
- Opcional: retry/backoff configurável para jobs/pipelines.

**Labels sugeridas:** `feature`, `daemon`, `operations`

---

## Issue O2 — Logs e rastreio (RUST_LOG, job/pipeline)

**Título:** `feat(daemon): padrão de logs (RUST_LOG) e correlação job/pipeline`

**Descrição:**

Estabelecer padrão de logs para o daemon (RUST_LOG e níveis por módulo) e correlação de logs por job ou pipeline (ex.: request_id ou job_id em cada log), para facilitar debug e análise em produção.

- Documentar níveis recomendados de RUST_LOG por ambiente (dev vs prod).
- Incluir em cada log de job/pipeline um identificador (job_id, pipeline_id) para correlacionar linhas.
- Referência a [observability](../features/observability.md) se logs forem integrados a OpenTelemetry/traces.

**Critério de aceite:**

- Doc descreve RUST_LOG e boas práticas; logs de jobs/pipelines incluem identificador de correlação.
- Opcional: integração com tracing existente (observability.md).

**Labels sugeridas:** `feature`, `daemon`, `operations`

---

## Issue O3 — Métricas e alertas (jobs concluídos, falhas, custo)

**Título:** `feat(daemon): expor métricas de jobs (concluídos, falhas, custo) para dashboard ou alertas`

**Descrição:**

Expor métricas operacionais do daemon (jobs concluídos, falhas, custo por run) para dashboard ou sistema de alertas. Alinhar ao que já existe em [observability](../features/observability.md) (OpenTelemetry, Prometheus) quando fizer sentido.

- Métricas: contadores de jobs concluídos, falhas, tempo de execução; custo (tokens/custo estimado) por pipeline ou por run.
- Exportar via OTLP/Prometheus ou endpoint simples, conforme padrão do projeto.
- Documentar quais métricas estão disponíveis e como conectar a Grafana/alertas (opcional).

**Critério de aceite:**

- Métricas de jobs (e opcionalmente custo) expostas; doc descreve como consumir (dashboard/alertas).
- Consistente com observability.md quando aplicável.

**Labels sugeridas:** `feature`, `daemon`, `operations`

---

## Ordem sugerida

| Ordem | Issue | Motivo |
|-------|--------|--------|
| 1 | O1 (Health check) | Pré-requisito para deploy confiável. |
| 2 | O2 (Logs e correlação) | Debug e análise. |
| 3 | O3 (Métricas/alertas) | Visibilidade e alertas. |

---

**Última atualização:** 2026-03-11
