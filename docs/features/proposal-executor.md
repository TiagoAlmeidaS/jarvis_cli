# Proposal Executor — Fechar o Ciclo Proposta -> Execucao

**Status**: Implementado
**Prioridade**: CRITICA
**Gap**: G1
**Roadmap**: Fase 1, Step 1.1
**Ultima atualizacao**: 2026-02-13

---

## 1. Visao Geral

O `ProposalExecutor` e o componente que falta para fechar o loop autonomo do daemon.
Hoje o fluxo e:

```
Metrics Collector → Strategy Analyzer → Proposals (armazenadas) → ??? → Nada acontece
```

Com o executor, o fluxo completo sera:

```
Metrics Collector → Strategy Analyzer → Proposals → [Aprovacao] → Proposal Executor → Acao Real
```

O executor le propostas com status `approved` e aplica as mudancas no sistema
(criar pipeline, mudar frequencia, adicionar source, etc.).

---

## 2. Motivacao

Sem o executor, o daemon e um **analisador passivo** — ele sugere mas nunca faz.
Isso quebra fundamentalmente o conceito de agente autonomo. O usuario precisa
manualmente interpretar cada proposta e aplicar mudancas via CLI, o que anula
a automacao.

**Antes**: Usuario precisa ler propostas + executar manualmente
**Depois**: Daemon executa automaticamente propostas aprovadas (ou auto-aprovadas)

---

## 3. Arquitetura

### 3.1 Fluxo de Execucao

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────────┐
│  Scheduler Tick  │────▶│ Proposal Executor│────▶│ Aplicar Mudanca     │
│  (a cada minuto) │     │                  │     │                     │
└─────────────────┘     │ 1. Query approved │     │ - CreatePipeline    │
                        │    proposals      │     │ - ModifyPipeline    │
                        │ 2. Validate       │     │ - DisablePipeline   │
                        │ 3. Execute action │     │ - ChangeFrequency   │
                        │ 4. Mark executed  │     │ - AddSource         │
                        │    or failed      │     │ - RemoveSource      │
                        └──────────────────┘     │ - ChangeModel       │
                                                  │ - ScaleUp/Down      │
                                                  └─────────────────────┘
```

### 3.2 Componente Principal

```rust
/// Executes approved proposals by applying their actions to the system.
pub struct ProposalExecutor {
    db: Arc<DaemonDb>,
}

impl ProposalExecutor {
    /// Check for approved proposals and execute them.
    pub async fn execute_pending(&self) -> Result<ExecutionSummary> {
        let approved = self.db.list_proposals(&ProposalFilter {
            status: Some(ProposalStatus::Approved),
            ..Default::default()
        }).await?;

        let mut summary = ExecutionSummary::default();

        for proposal in approved {
            match self.execute_proposal(&proposal).await {
                Ok(()) => {
                    self.db.mark_proposal_executed(&proposal.id).await?;
                    summary.executed += 1;
                }
                Err(e) => {
                    self.db.mark_proposal_failed(&proposal.id, &e.to_string()).await?;
                    summary.failed += 1;
                }
            }
        }

        summary
    }

    /// Execute a single proposal based on its action_type.
    async fn execute_proposal(&self, proposal: &DaemonProposal) -> Result<()> {
        match proposal.action_type {
            ActionType::CreatePipeline => self.create_pipeline(proposal).await,
            ActionType::ModifyPipeline => self.modify_pipeline(proposal).await,
            ActionType::DisablePipeline => self.disable_pipeline(proposal).await,
            ActionType::ChangeFrequency => self.change_frequency(proposal).await,
            ActionType::ChangeNiche => self.change_niche(proposal).await,
            ActionType::AddSource => self.add_source(proposal).await,
            ActionType::RemoveSource => self.remove_source(proposal).await,
            ActionType::ScaleUp => self.scale_up(proposal).await,
            ActionType::ScaleDown => self.scale_down(proposal).await,
            ActionType::ChangeModel => self.change_model(proposal).await,
            ActionType::Custom => self.execute_custom(proposal).await,
        }
    }
}
```

### 3.3 Integracao com o Scheduler

O executor deve rodar como parte do tick do scheduler, **apos** os pipelines
normais serem verificados:

```rust
// Em scheduler.rs, no metodo tick():
async fn tick(&self) -> Result<()> {
    // 1. Check pipelines and enqueue jobs (existente)
    self.check_pipelines().await?;

    // 2. Execute approved proposals (NOVO)
    let summary = self.proposal_executor.execute_pending().await?;
    if summary.executed > 0 || summary.failed > 0 {
        info!(
            "Proposal execution: {} executed, {} failed",
            summary.executed, summary.failed
        );
    }

    // 3. Expire stale proposals (NOVO)
    let expired = self.db.expire_proposals().await?;
    if expired > 0 {
        info!("{expired} proposals expired");
    }

    Ok(())
}
```

---

## 4. Mapeamento ActionType -> Acao Concreta

| ActionType | Acao no Banco | Campos usados de `proposed_config` |
|------------|---------------|-------------------------------------|
| `CreatePipeline` | `INSERT INTO daemon_pipelines` | `{id, name, strategy, config_json, schedule_cron}` |
| `ModifyPipeline` | `UPDATE daemon_pipelines SET config_json` | `{config_json}` (merge com existente) |
| `DisablePipeline` | `UPDATE daemon_pipelines SET enabled = 0` | `{pipeline_id}` (do campo `proposal.pipeline_id`) |
| `ChangeFrequency` | `UPDATE daemon_pipelines SET schedule_cron` | `{schedule_cron}` |
| `ChangeNiche` | `UPDATE config_json.seo.niche` | `{niche, target_audience, keywords}` |
| `AddSource` | `INSERT INTO daemon_sources` | `{source_type, name, url, scrape_selector}` |
| `RemoveSource` | `DELETE FROM daemon_sources WHERE id` | `{source_id}` |
| `ScaleUp` | Aumentar frequencia ou adicionar mais sources | Contexto-dependente |
| `ScaleDown` | Diminuir frequencia ou desabilitar pipeline | Contexto-dependente |
| `ChangeModel` | `UPDATE config_json.llm.model` | `{provider, model}` |
| `Custom` | Log a acao + notificar usuario | `{description}` |

---

## 5. Seguranca e Guardrails

### 5.1 Validacoes antes da execucao

1. **Proposta nao expirada**: Verificar `expires_at > now()`
2. **Pipeline existe** (para modify/disable): Verificar `pipeline_id` valido
3. **Config valida**: Deserializar e validar `proposed_config` antes de aplicar
4. **Sem conflito**: Nao executar se ja existe proposta em execucao para o mesmo pipeline
5. **Rate limit**: Maximo de N execucoes por tick (evitar burst de mudancas)

### 5.2 Rollback

Para acoes destrutivas (disable, remove_source), salvar o estado anterior
no campo `metrics_snapshot` da proposta antes de executar, permitindo
reverter se necessario.

### 5.3 Notificacoes

Apos executar uma proposta, registrar em `daemon_logs` com nivel `Info`:
```
Proposal executed: "Increase frequency to 4x/day" (seo-concursos)
```

Para falhas, nivel `Error`:
```
Proposal execution failed: "Add new RSS source" - invalid URL format
```

---

## 6. Novos metodos no DaemonDb

```rust
/// List proposals ready for execution (approved, not expired, not already executed).
pub async fn list_executable_proposals(&self) -> Result<Vec<DaemonProposal>>;

/// Mark a proposal as executed with timestamp.
/// Ja existe: mark_proposal_executed(&self, id: &str) -> Result<()>

/// Mark a proposal as failed with error message.
/// Ja existe: mark_proposal_failed(&self, id: &str, error: &str) -> Result<()>

/// Update a pipeline's config_json (merge).
pub async fn update_pipeline_config(&self, id: &str, config: &serde_json::Value) -> Result<()>;

/// Update a pipeline's schedule_cron.
pub async fn update_pipeline_schedule(&self, id: &str, cron: &str) -> Result<()>;

/// Enable/disable a pipeline.
pub async fn set_pipeline_enabled(&self, id: &str, enabled: bool) -> Result<()>;
```

---

## 7. CLI: Visualizacao de execucoes

Estender `jarvis daemon proposals list` para mostrar status `executed` e `failed`:

```
$ jarvis daemon proposals list --status executed

 ID        Pipeline          Action           Title                     Executed At
 ──────────────────────────────────────────────────────────────────────────────────
 a1b2c3d4  seo-concursos     change_frequency Increase to 4x/day       2026-02-13 03:15
 e5f6g7h8  seo-tech          add_source       Add .NET Blog RSS feed   2026-02-13 03:15
```

---

## 8. Testes

| Teste | Tipo | Descricao |
|-------|------|-----------|
| `test_execute_create_pipeline` | Unitario | Cria pipeline a partir de proposta |
| `test_execute_modify_pipeline` | Unitario | Altera config de pipeline existente |
| `test_execute_disable_pipeline` | Unitario | Desabilita pipeline |
| `test_execute_add_source` | Unitario | Adiciona source a um pipeline |
| `test_execute_invalid_config` | Unitario | Falha graciosamente com config invalida |
| `test_execute_expired_proposal` | Unitario | Ignora propostas expiradas |
| `test_execute_marks_status` | Unitario | Verifica status executed/failed no banco |
| `test_scheduler_integrates_executor` | Integracao | Verifica que scheduler chama executor |

---

## 9. Arquivos a criar/modificar

| Arquivo | Acao | Descricao |
|---------|------|-----------|
| `daemon/src/executor.rs` | **Criar** | `ProposalExecutor` struct e logica |
| `daemon/src/scheduler.rs` | Modificar | Integrar executor no tick() |
| `daemon/src/main.rs` | Modificar | Instanciar executor e passar ao scheduler |
| `daemon-common/src/db.rs` | Modificar | Novos metodos de update de pipeline |
| `cli/src/daemon_cmd.rs` | Modificar | Mostrar propostas executadas no status |

---

## 10. Estimativa

- **Complexidade**: Media
- **Tempo estimado**: 2-3 dias
- **Risco**: Baixo (operacoes CRUD bem definidas, sem dependencia externa)
- **Prerequisito**: Nenhum
