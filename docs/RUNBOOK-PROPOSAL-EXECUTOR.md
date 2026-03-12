# Runbook: Validar que propostas aprovadas são executadas

Checklist para validar o loop **Strategy Analyzer → Proposals → Executor → Ação** em ambiente real: uma proposta aprovada é executada pelo daemon e o estado do sistema é alterado. Ver issue #34 e [proposal-executor.md](features/proposal-executor.md).

---

## Pré-requisitos

- Daemon rodando (Docker ou local) com banco persistente (ex.: `~/.jarvis/daemon.db` ou `JARVIS_DAEMON_DB` apontando para o DB do container).
- CLI Jarvis configurada para o mesmo DB (ou use `jarvis daemon` no host que acessa o mesmo volume/DB).

---

## Opção A: Proposta gerada pelo Strategy Analyzer

1. **Tenha pelo menos um pipeline habilitado** (ex.: SEO blog ou metrics_collector) para o strategy_analyzer ter contexto.
2. **Configure o pipeline strategy_analyzer** e deixe-o rodar no cron (ou dispare um job manualmente, se o CLI expuser isso). O analyzer gera propostas (regras ou LLM) e pode auto-aprovar as de baixo risco.
3. **Liste propostas pendentes:**
   ```bash
   jarvis daemon proposals list
   ```
4. **Aprove uma proposta pendente** (se ainda não estiver aprovada):
   ```bash
   jarvis daemon proposals approve <id>
   ```
   Use o ID completo ou os primeiros 8 caracteres.
5. **Aguarde o próximo tick do scheduler** (até 60 s com intervalo padrão). O executor roda a cada tick e processa propostas aprovadas.
6. **Verifique execução:**
   ```bash
   jarvis daemon proposals list --all
   ```
   Procure a proposta com status `executed`. Confira no DB ou via CLI que o recurso foi alterado (ex.: novo pipeline criado, pipeline desabilitado, source adicionada).

---

## Opção B: Proposta inserida manualmente (teste controlado)

Para validar sem depender do strategy_analyzer ou LLM:

1. **Inserir uma proposta aprovada no SQLite.** O daemon usa a tabela `daemon_proposals`. Campos mínimos (exemplo para `create_pipeline`):
   - `id`: UUID ou string única
   - `pipeline_id`: NULL para CreatePipeline
   - `action_type`: `create_pipeline`
   - `title`, `description`, `reasoning`: texto
   - `confidence`: 0.0–1.0
   - `risk_level`: `low` | `medium` | `high`
   - `status`: `approved`
   - `proposed_config`: JSON (ex.: `{"id":"test-runbook","name":"Test","strategy":"seo_blog","schedule_cron":"0 9 * * *","config_json":{}}`)
   - `auto_approvable`: 0
   - `created_at`, `expires_at`: Unix timestamp (expires_at opcional, ou 24 h à frente)

   Ou use a API/CLI se existir comando de criação de proposta no futuro.

2. **Aprovar via CLI** (se a proposta foi criada como `pending`):
   ```bash
   jarvis daemon proposals approve <id>
   ```

3. **Aguardar um tick** (até 60 s) ou **reiniciar o daemon** para forçar um tick logo após o start.

4. **Verificar:**
   ```bash
   jarvis daemon proposals list --all
   ```
   Status da proposta deve ser `executed`. Para CreatePipeline, verificar que o pipeline existe: `jarvis daemon pipeline list` (ou equivalente).

---

## Verificação rápida (teste automatizado)

Sem daemon real, o fluxo é validado pelos testes de integração:

```bash
cd jarvis-rs
cargo test -p jarvis-daemon --test proposal_executor_e2e
```

Isso executa: proposta aprovada → `ProposalExecutor::execute_pending()` → assert de estado (pipeline criado ou desabilitado) e status da proposta `executed`. Ver [proposal-executor.md](features/proposal-executor.md) § Validação e testes.

---

## Referências

- [proposal-executor.md](features/proposal-executor.md) — Projeto e validação do executor
- [autonomy-loop-gaps.md](issues/autonomy-loop-gaps.md) — G1 (Proposal Executor)
- [DAEMON_QUICK_START.md](DAEMON_QUICK_START.md) — Comandos do daemon
