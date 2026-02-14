# Goal System — Metas Mensuraveis para o Daemon

**Status**: Implementado
**Prioridade**: CRITICA
**Gap**: G2
**Roadmap**: Fase 1, Step 1.2
**Ultima atualizacao**: 2026-02-13

---

## 1. Visao Geral

O Goal System da ao daemon **proposito e direcao**. Sem metas explicitas, o
`strategy_analyzer` nao tem criterio para priorizar propostas — ele sugere
melhorias genericas sem saber o que o usuario realmente quer.

Com goals, o analyzer pode:
- Priorizar acoes que avancam as metas
- Medir progresso quantitativamente
- Alertar quando metas estao em risco
- Auto-ajustar estrategias baseado em distance-to-goal

### Exemplo

```
Meta: "Revenue >= $200/mes"
  Estado atual: $45/mes
  Gap: $155
  → Analyzer prioriza propostas que aumentam revenue (mais artigos, melhores nichos)
  → Descarta propostas de "escalar para baixo"

Meta: "Publicar >= 90 artigos/mes"
  Estado atual: 62/mes
  Gap: 28 artigos
  → Analyzer sugere aumentar frequencia ou adicionar pipelines
```

---

## 2. Modelo de Dados

### 2.1 Tabela `daemon_goals`

```sql
CREATE TABLE IF NOT EXISTS daemon_goals (
    id              TEXT PRIMARY KEY,            -- UUID
    name            TEXT NOT NULL,               -- ex: "Revenue Mensal"
    description     TEXT,                        -- descricao humana
    metric_type     TEXT NOT NULL CHECK(metric_type IN (
        'revenue', 'content_count', 'pageviews', 'clicks',
        'ctr', 'subscribers', 'cost_limit', 'custom'
    )),
    target_value    REAL NOT NULL,               -- valor meta (ex: 200.0)
    target_unit     TEXT NOT NULL DEFAULT 'USD', -- unidade (USD, count, percent, etc.)
    period          TEXT NOT NULL DEFAULT 'monthly' CHECK(period IN (
        'daily', 'weekly', 'monthly', 'quarterly', 'yearly'
    )),
    pipeline_id     TEXT,                        -- NULL = global, ou vinculado a pipeline especifica
    current_value   REAL DEFAULT 0.0,            -- ultimo valor medido
    last_measured   INTEGER,                     -- unix timestamp da ultima medicao
    status          TEXT NOT NULL DEFAULT 'active' CHECK(status IN (
        'active', 'achieved', 'paused', 'failed', 'archived'
    )),
    priority        INTEGER NOT NULL DEFAULT 1,  -- 1=highest, 5=lowest
    deadline        INTEGER,                     -- unix timestamp (opcional)
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_goals_status ON daemon_goals(status);
CREATE INDEX IF NOT EXISTS idx_goals_pipeline ON daemon_goals(pipeline_id);
```

### 2.2 Tabela `daemon_goal_snapshots` (historico)

```sql
CREATE TABLE IF NOT EXISTS daemon_goal_snapshots (
    id              TEXT PRIMARY KEY,
    goal_id         TEXT NOT NULL REFERENCES daemon_goals(id),
    measured_value  REAL NOT NULL,
    target_value    REAL NOT NULL,
    gap             REAL NOT NULL,               -- target - measured
    progress_pct    REAL NOT NULL,               -- (measured / target) * 100
    period_start    INTEGER NOT NULL,
    period_end      INTEGER NOT NULL,
    created_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_goal_snapshots_goal ON daemon_goal_snapshots(goal_id, created_at DESC);
```

### 2.3 Structs Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonGoal {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub metric_type: GoalMetricType,
    pub target_value: f64,
    pub target_unit: String,
    pub period: GoalPeriod,
    pub pipeline_id: Option<String>,
    pub current_value: f64,
    pub last_measured: Option<i64>,
    pub status: GoalStatus,
    pub priority: i32,
    pub deadline: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalMetricType {
    Revenue,
    ContentCount,
    Pageviews,
    Clicks,
    Ctr,
    Subscribers,
    CostLimit,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Achieved,
    Paused,
    Failed,
    Archived,
}

/// Summary of goal progress for the strategy analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub goal: DaemonGoal,
    pub gap: f64,               // target - current
    pub progress_pct: f64,      // (current / target) * 100
    pub on_track: bool,         // estimativa se vai atingir no prazo
    pub days_remaining: Option<i64>,
}
```

---

## 3. Integracao com Strategy Analyzer

### 3.1 O analyzer usa goals no prompt

O `gather_analysis_data()` no `strategy_analyzer.rs` deve incluir goals:

```rust
// Em gather_analysis_data():
let goals = db.list_goals(GoalStatus::Active).await?;
data.push_str("\n## Active Goals\n");
for goal in &goals {
    let progress = calculate_progress(&goal);
    data.push_str(&format!(
        "- {} (priority {}): {:.1}/{:.1} {} ({:.0}% complete, {})\n",
        goal.name,
        goal.priority,
        goal.current_value,
        goal.target_value,
        goal.target_unit,
        progress.progress_pct,
        if progress.on_track { "ON TRACK" } else { "AT RISK" },
    ));
}
```

### 3.2 O system prompt instrui o LLM a considerar goals

Adicionar ao `build_system_prompt()`:

```
GOALS:
- Proposals MUST align with the active goals listed below
- Prioritize actions that close the largest gaps first
- If a goal is "AT RISK", suggest urgent actions to recover
- Never suggest actions that conflict with higher-priority goals
- If all goals are on track, suggest optimizations or new goals
```

### 3.3 O metrics_collector atualiza current_value

O `metrics_collector` pipeline deve, alem de registrar metricas, atualizar
o `current_value` dos goals correspondentes:

```rust
// Em metrics_collector.rs execute():
// Apos coletar metricas, atualizar goals.
let goals = db.list_goals(GoalStatus::Active).await?;
for goal in &goals {
    let current = match goal.metric_type {
        GoalMetricType::Revenue => calculate_period_revenue(db, &goal).await?,
        GoalMetricType::ContentCount => count_period_content(db, &goal).await?,
        GoalMetricType::Pageviews => get_period_pageviews(db, &goal).await?,
        _ => continue,
    };
    db.update_goal_current_value(&goal.id, current).await?;
    db.create_goal_snapshot(&goal.id, current, goal.target_value).await?;
}
```

---

## 4. CLI: `jarvis daemon goals`

```
jarvis daemon goals                           # Lista goals ativos com progresso
jarvis daemon goals add                       # Adiciona novo goal interativamente
jarvis daemon goals add --name "Revenue" \
    --metric revenue --target 200 --unit USD \
    --period monthly --priority 1             # Adiciona via flags
jarvis daemon goals show <id>                 # Detalhes + historico de snapshots
jarvis daemon goals update <id> --target 300  # Atualiza meta
jarvis daemon goals pause <id>                # Pausa goal
jarvis daemon goals archive <id>              # Arquiva goal
jarvis daemon goals progress                  # Dashboard de progresso resumido
```

### Exemplo de output `jarvis daemon goals`

```
 Active Goals                                          Progress
 ──────────────────────────────────────────────────────────────
 P1  Revenue Mensal       $45.20 / $200.00  USD    ██░░░░░░░░  22.6%  AT RISK
 P1  Artigos Publicados   62 / 90           count  ██████░░░░  68.9%  ON TRACK
 P2  Custo LLM Maximo     $2.30 / $5.00     USD    ████░░░░░░  46.0%  OK
 P3  Pageviews Mensais    3,200 / 10,000    count  ███░░░░░░░  32.0%  AT RISK
```

### Exemplo de output `jarvis daemon goals progress`

```
 Goal Summary (February 2026)
 ──────────────────────────────────────────────────────────────
 Total goals:     4 active, 1 achieved, 0 failed
 On track:        2/4 (50%)
 At risk:         2/4 — Revenue Mensal, Pageviews Mensais
 Days remaining:  15

 Recommendations:
  ! Revenue is 77% behind target — analyzer should prioritize revenue actions
  ! Pageviews gap of 6,800 — consider adding more sources or new pipelines
  ✓ Article production on track to meet 90/month target
  ✓ LLM costs well within budget
```

---

## 5. Seguranca

- Goals sao **opcionais**: se nenhum goal existe, o analyzer funciona como antes
- Goals nunca executam acoes diretamente — eles apenas **informam** o analyzer
- `CostLimit` goals podem gerar alertas se o daemon estiver gastando demais
- Deadline expirado muda status para `failed` automaticamente

---

## 6. Testes

| Teste | Tipo | Descricao |
|-------|------|-----------|
| `test_goal_crud` | Unitario | CRUD basico de goals |
| `test_goal_progress_calculation` | Unitario | Calculo de gap e progress_pct |
| `test_goal_snapshot_history` | Unitario | Snapshots salvos corretamente |
| `test_metrics_collector_updates_goals` | Integracao | Metrics collector atualiza current_value |
| `test_analyzer_includes_goals` | Integracao | Analysis data inclui goals no prompt |
| `test_deadline_expiration` | Unitario | Goal com deadline expirado → status failed |

---

## 7. Arquivos a criar/modificar

| Arquivo | Acao | Descricao |
|---------|------|-----------|
| `daemon-common/src/models.rs` | Modificar | Adicionar enums e structs de Goal |
| `daemon-common/src/db.rs` | Modificar | Migration + CRUD de goals e snapshots |
| `daemon/src/pipelines/metrics_collector.rs` | Modificar | Atualizar goals apos coletar metricas |
| `daemon/src/pipelines/strategy_analyzer.rs` | Modificar | Incluir goals no prompt do LLM |
| `cli/src/daemon_cmd.rs` | Modificar | Subcomando `goals` |

---

## 8. Estimativa

- **Complexidade**: Baixa-Media
- **Tempo estimado**: 1-2 dias
- **Risco**: Baixo
- **Prerequisito**: Nenhum (pode ser implementado em paralelo com Proposal Executor)
