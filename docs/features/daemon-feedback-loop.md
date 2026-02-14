# Daemon Feedback Loop — Observacao, Analise e Aprovacao

**Status**: Em Progresso
**Prioridade**: Alta
**Versao**: 1.0.0
**Ultima atualizacao**: 2026-02-13

---

## 1. Visao Geral

Este documento descreve as Fases 3-5 do roadmap de automacao do Jarvis Daemon,
transformando-o de um **executor de pipelines** em um **agente autonomo com
feedback loop**:

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ OBSERVE  │───▶│ ANALYZE  │───▶│ PROPOSE  │───▶│ EXECUTE  │
│          │    │          │    │          │    │          │
│ Coleta   │    │ LLM avalia│   │ Gera plano│   │ Pipelines│
│ metricas │    │ e compara │    │ pede OK   │    │ rodam    │
│ revenue  │    │ tendencias│    │ do usuario│    │          │
└──────────┘    └──────────┘    └──────────┘    └──────────┘
      ▲                                               │
      └───────────────────────────────────────────────┘
                       FEEDBACK LOOP
```

### Diferenca fundamental

| Antes (Fase 1-2) | Depois (Fase 3-5) |
|---|---|
| Cron dispara → pipeline roda → fim | Cron dispara → coleta metricas → LLM analisa → propoe acao → usuario aprova → executa → mede resultado |
| Sem visibilidade de resultado | Dashboard de revenue, clicks, CTR |
| Estrategia fixa | Estrategia adaptativa (mais do que funciona, menos do que nao) |
| Tudo automatico sem controle | Workflow de aprovacao com auto-approve para low-risk |

---

## 2. Novas Tabelas SQLite

### daemon_proposals

Acoes propostas pelo agente que aguardam aprovacao do usuario.

```sql
CREATE TABLE IF NOT EXISTS daemon_proposals (
    id              TEXT PRIMARY KEY,
    pipeline_id     TEXT REFERENCES daemon_pipelines(id),
    action_type     TEXT NOT NULL CHECK(action_type IN (
        'create_pipeline', 'modify_pipeline', 'disable_pipeline',
        'change_niche', 'change_frequency', 'add_source', 'remove_source',
        'scale_up', 'scale_down', 'change_model', 'custom'
    )),
    title           TEXT NOT NULL,
    description     TEXT NOT NULL,
    reasoning       TEXT NOT NULL,       -- LLM chain-of-thought
    confidence      REAL NOT NULL,       -- 0.0 a 1.0
    risk_level      TEXT NOT NULL CHECK(risk_level IN ('low', 'medium', 'high')),
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending', 'approved', 'rejected', 'expired', 'executed', 'failed')),
    proposed_config TEXT,                -- JSON com a configuracao proposta
    metrics_snapshot TEXT,               -- JSON com metricas no momento da proposta
    auto_approvable INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    reviewed_at     INTEGER,
    executed_at     INTEGER,
    expires_at      INTEGER              -- proposta expira se nao revisada
);
```

### daemon_revenue

Rastreamento de receita real por conteudo/pipeline.

```sql
CREATE TABLE IF NOT EXISTS daemon_revenue (
    id              TEXT PRIMARY KEY,
    content_id      TEXT REFERENCES daemon_content(id),
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    source          TEXT NOT NULL CHECK(source IN (
        'adsense', 'affiliate', 'gumroad', 'stripe', 'manual', 'estimated'
    )),
    amount          REAL NOT NULL,
    currency        TEXT NOT NULL DEFAULT 'USD',
    period_start    INTEGER NOT NULL,
    period_end      INTEGER NOT NULL,
    external_id     TEXT,                -- ID da transacao no sistema externo
    metadata_json   TEXT,
    created_at      INTEGER NOT NULL
);
```

---

## 3. Novos Modelos (daemon-common)

### Enums

- `ActionType` — tipo de acao proposta (create_pipeline, scale_up, etc.)
- `RiskLevel` — low, medium, high
- `ProposalStatus` — pending, approved, rejected, expired, executed, failed
- `RevenueSource` — adsense, affiliate, gumroad, stripe, manual, estimated

### Structs

- `DaemonProposal` — row model para daemon_proposals
- `DaemonRevenue` — row model para daemon_revenue
- `CreateProposal` — input para criar proposta
- `ProposalFilter` — filtros para listar propostas
- `RevenueSummary` — agregacao de receita (total, por pipeline, por periodo)

---

## 4. Novos Pipelines (daemon)

### 4.1 metrics_collector

Pipeline que roda periodicamente (1x/dia) para coletar metricas:

1. **Google Search Console API** (gratis) → impressoes, clicks, CTR, posicao media
2. **Estimativa de revenue** → baseada em clicks * CPC medio do nicho
3. **WordPress stats** (opcional) → pageviews por artigo

Config:
```json
{
  "strategy": "metrics_collector",
  "schedule_cron": "0 6 * * *",
  "config": {
    "google_search_console": {
      "site_url": "https://seu-blog.com",
      "credentials_path": "~/.jarvis/gsc-credentials.json"
    },
    "estimated_cpc": {
      "default": 0.05,
      "by_niche": { "concursos": 0.15, "tech": 0.08 }
    }
  }
}
```

### 4.2 strategy_analyzer

Pipeline que roda semanalmente para analisar metricas e propor acoes:

1. Le metricas dos ultimos 7/30 dias do `daemon_metrics` e `daemon_revenue`
2. Compara performance entre pipelines/nichos
3. Envia para o LLM com prompt estruturado
4. LLM retorna `ProposedAction[]` com confidence e reasoning
5. Persiste como `daemon_proposals` com status `pending`

Config:
```json
{
  "strategy": "strategy_analyzer",
  "schedule_cron": "0 8 * * 1",
  "config": {
    "llm": { "provider": "openrouter", "model": "qwen/qwen3-coder-next" },
    "analysis_window_days": 30,
    "min_confidence_for_auto_approve": 0.85,
    "max_auto_approve_risk": "low"
  }
}
```

---

## 5. Workflow de Aprovacao

### CLI Commands

```bash
# Listar propostas pendentes
jarvis daemon proposals

# Ver detalhes de uma proposta
jarvis daemon proposals show <id>

# Aprovar
jarvis daemon proposals approve <id>

# Rejeitar com motivo
jarvis daemon proposals reject <id> --reason "nao quero mudar de nicho agora"

# Auto-approve: configurar regras
jarvis daemon proposals auto-approve --max-risk low --min-confidence 0.85
```

### Fluxo

```
LLM analisa metricas
     │
     ▼
Gera ProposedAction
     │
     ├── confidence >= 0.85 AND risk == low AND auto_approvable?
     │       │
     │       ├── Sim → status = approved → executa automaticamente
     │       │
     │       └── Nao → status = pending → aguarda usuario
     │
     ▼
Usuario no CLI: jarvis daemon proposals
     │
     ├── approve → status = approved → scheduler executa
     │
     └── reject → status = rejected → registra feedback
```

---

## 6. Implementacao (neste PR)

### Fase 3A — Fundacao (database + models)
- [x] Tabelas `daemon_proposals` e `daemon_revenue` no migration SQL
- [x] Modelos e enums em `daemon-common/src/models.rs`
- [x] CRUD no `DaemonDb`
- [x] Testes unitarios

### Fase 3B — CLI de aprovacao
- [x] `jarvis daemon proposals` (list/show/approve/reject)
- [x] `jarvis daemon revenue` (summary)

### Fase 3C — Pipelines de observacao (futuro PR)
- [ ] `metrics_collector` pipeline (Google Search Console integration)
- [ ] `strategy_analyzer` pipeline (LLM analysis + proposal generation)

---

## 7. Metricas de Sucesso

| Metrica | Target Fase 3 |
|---------|--------------|
| Proposals geradas/semana | 3-5 |
| Tempo medio de revisao | < 24h |
| Taxa de aprovacao | 60-80% |
| Revenue tracking accuracy | ±20% |
